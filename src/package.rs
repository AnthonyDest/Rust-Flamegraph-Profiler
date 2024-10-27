use super::checksum::Checksum;
use super::Event;
use crossbeam::channel::Sender;
use std::fs;
use std::sync::{Arc, Mutex};

pub struct Package {
    pub name: String,
}

pub struct PackageDownloader {
    pkg_start_idx: usize,
    num_pkgs: usize,
    // event_sender: Sender<Event>,
    pkg_send: Sender<Package>,
    package_lines: Arc<Vec<String>>,
}

impl PackageDownloader {
    pub fn new(
        pkg_start_idx: usize,
        num_pkgs: usize,
        // event_sender: Sender<Event>,
        pkg_send: Sender<Package>,
        package_lines: Arc<Vec<String>>,
    ) -> Self {
        Self {
            pkg_start_idx,
            num_pkgs,
            pkg_send,
            package_lines,
        }
    }

    pub fn run(&self, pkg_checksum: Arc<Mutex<Checksum>>) {
        // Generate a set of packages and place them into the event queue
        // Update the package checksum with each package name

        let number_of_lines = self.package_lines.len();

        // compute checksum before acquiring mutex, update is faster than compute
        let mut batch_checksum = Checksum::default();
        for i in 0..self.num_pkgs {
            let name = self.package_lines[(self.pkg_start_idx + i) % number_of_lines].clone();
            batch_checksum.update(Checksum::with_sha256(&name));
            self.pkg_send.send(Package { name }).unwrap();
        }
        pkg_checksum.lock().unwrap().update(batch_checksum);
    }
}
