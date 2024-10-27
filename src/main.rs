#![warn(clippy::all)]
use crossbeam::channel::{unbounded, Receiver, Sender};
use lab4::idea::Idea;
use lab4::package::Package;
use lab4::{
    checksum::Checksum, idea::IdeaGenerator, package::PackageDownloader, student::Student, Event,
};
use std::env;
use std::error::Error;
use std::fs;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

struct Args {
    pub num_ideas: usize,
    pub num_idea_gen: usize,
    pub num_pkgs: usize,
    pub num_pkg_gen: usize,
    pub num_students: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let num_ideas = args.get(1).map_or(Ok(80), |a| a.parse())?;
    let num_idea_gen = args.get(2).map_or(Ok(2), |a| a.parse())?;
    let num_pkgs = args.get(3).map_or(Ok(4000), |a| a.parse())?;
    let num_pkg_gen = args.get(4).map_or(Ok(6), |a| a.parse())?;
    let num_students = args.get(5).map_or(Ok(6), |a| a.parse())?;
    let args = Args {
        num_ideas,
        num_idea_gen,
        num_pkgs,
        num_pkg_gen,
        num_students,
    };

    hackathon(&args);
    Ok(())
}

fn per_thread_amount(thread_idx: usize, total: usize, threads: usize) -> usize {
    let per_thread = total / threads;
    let extras = total % threads;
    per_thread + (thread_idx < extras) as usize
}

fn hackathon(args: &Args) {
    // Use message-passing channel to queue each event
    let (pkg_send, pkg_recv) = unbounded::<Package>();
    let (idea_send, idea_recv) = unbounded::<Idea>();
    let (out_of_idea_send, out_of_idea_recv) = unbounded::<bool>();

    let mut threads = vec![];
    // Checksums of all the generated ideas and packages
    let mut idea_checksum = Arc::new(Mutex::new(Checksum::default()));
    let mut pkg_checksum = Arc::new(Mutex::new(Checksum::default()));
    // Checksums of the ideas and packages used by students to build ideas. Should match the
    // previous checksums.
    let mut student_idea_checksum = Arc::new(Mutex::new(Checksum::default()));
    let mut student_pkg_checksum = Arc::new(Mutex::new(Checksum::default()));

    // Spawn student threads
    for i in 0..args.num_students {
        let mut student = Student::new(
            i,
            Receiver::clone(&idea_recv),
            Receiver::clone(&pkg_recv),
            Receiver::clone(&out_of_idea_recv),
        );
        let student_idea_checksum = Arc::clone(&student_idea_checksum);
        let student_pkg_checksum = Arc::clone(&student_pkg_checksum);
        let thread = spawn(move || student.run(student_idea_checksum, student_pkg_checksum));
        threads.push(thread);
    }

    // Spawn package downloader threads. Packages are distributed evenly across threads.
    // read in beforehand and make into an array, share to all threads
    let package_lines = fs::read_to_string("data/packages.txt")
        .expect("Failed to read packages file")
        .lines()
        .map(|line| line.to_owned())
        .collect::<Vec<String>>();

    let package_lines_shared = Arc::new(package_lines);

    let mut start_idx = 0;
    for i in 0..args.num_pkg_gen {
        let num_pkgs = per_thread_amount(i, args.num_pkgs, args.num_pkg_gen);
        let downloader = PackageDownloader::new(
            start_idx,
            num_pkgs,
            Sender::clone(&pkg_send),
            package_lines_shared.clone(),
        );
        let pkg_checksum = Arc::clone(&pkg_checksum);
        start_idx += num_pkgs;

        let thread = spawn(move || downloader.run(pkg_checksum));
        threads.push(thread);
    }
    assert_eq!(start_idx, args.num_pkgs);

    // Spawn idea generator threads. Ideas and packages are distributed evenly across threads. In
    // each thread, packages are distributed evenly across ideas.

    // read in and compute crossproduct beforehand, share to all threads
    let products_lines = fs::read_to_string("data/ideas-products.txt").expect("file not found");
    let customers_lines = fs::read_to_string("data/ideas-customers.txt").expect("file not found");

    let idea_customer_cross_product: Vec<(String, String)> = products_lines
        .lines()
        .flat_map(|p| {
            customers_lines
                .lines()
                .map(move |c| (p.to_owned(), c.to_owned()))
        })
        .collect();

    let idea_customer_cross_product_shared = Arc::new(idea_customer_cross_product);

    let mut start_idx = 0;
    for i in 0..args.num_idea_gen {
        let num_ideas = per_thread_amount(i, args.num_ideas, args.num_idea_gen);
        let num_pkgs = per_thread_amount(i, args.num_pkgs, args.num_idea_gen);
        let num_students = per_thread_amount(i, args.num_students, args.num_idea_gen);
        let generator = IdeaGenerator::new(
            start_idx,
            num_ideas,
            num_students,
            num_pkgs,
            Sender::clone(&idea_send),
            Sender::clone(&out_of_idea_send),
            idea_customer_cross_product_shared.clone(),
        );
        let idea_checksum = Arc::clone(&idea_checksum);
        start_idx += num_ideas;

        let thread = spawn(move || generator.run(idea_checksum));
        threads.push(thread);
    }
    assert_eq!(start_idx, args.num_ideas);

    // Join all threads
    threads.into_iter().for_each(|t| t.join().unwrap());

    let idea = Arc::get_mut(&mut idea_checksum).unwrap().get_mut().unwrap();
    let student_idea = Arc::get_mut(&mut student_idea_checksum)
        .unwrap()
        .get_mut()
        .unwrap();
    let pkg = Arc::get_mut(&mut pkg_checksum).unwrap().get_mut().unwrap();
    let student_pkg = Arc::get_mut(&mut student_pkg_checksum)
        .unwrap()
        .get_mut()
        .unwrap();

    println!("Global checksums:\nIdea Generator: {}\nStudent Idea: {}\nPackage Downloader: {}\nStudent Package: {}", 
        idea, student_idea, pkg, student_pkg);
}
