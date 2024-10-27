use super::{checksum::Checksum, idea::Idea, package::Package, Event};
use crossbeam::channel::{Receiver, Sender};
use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};

pub struct Student {
    id: usize,
    idea: Option<Idea>,
    pkgs: Vec<Package>,
    skipped_idea: bool,
    idea_recv: Receiver<Idea>,
    pkg_recv: Receiver<Package>,
    is_out_of_ideas_recv: Receiver<bool>,
    idea_queue: Vec<Idea>,
}

impl Student {
    pub fn new(
        id: usize,
        idea_recv: Receiver<Idea>,
        pkg_recv: Receiver<Package>,
        is_out_of_ideas_recv: Receiver<bool>,
    ) -> Self {
        Self {
            id,
            idea_recv,
            pkg_recv,
            is_out_of_ideas_recv,
            idea: None,
            pkgs: vec![],
            skipped_idea: false,
            idea_queue: vec![],
        }
    }

    fn build_idea(
        &mut self,
        idea_checksum: &Arc<Mutex<Checksum>>,
        pkg_checksum: &Arc<Mutex<Checksum>>,
    ) {
        if let Some(ref idea) = self.idea {
            // Can only build ideas if we have acquired sufficient packages
            let pkgs_required = idea.num_pkg_required;

            // compute checksum before acquiring mutex, update is faster than compute
            let mut batch_idea_checksum = Checksum::default();
            batch_idea_checksum.update(Checksum::with_sha256(&idea.name));
            let mut idea_checksum = idea_checksum.lock().unwrap();
            idea_checksum.update(batch_idea_checksum);

            // compute checksum before acquiring mutex, update is faster than compute
            let pkgs_used = self.pkgs.drain(0..pkgs_required).collect::<Vec<_>>();
            let mut batch_pkg_checksum = Checksum::default();
            for pkg in pkgs_used.iter() {
                batch_pkg_checksum.update(Checksum::with_sha256(&pkg.name));
            }
            let mut pkg_checksum = pkg_checksum.lock().unwrap();
            pkg_checksum.update(batch_pkg_checksum);

            self.idea = None;
        }
    }

    pub fn run(&mut self, idea_checksum: Arc<Mutex<Checksum>>, pkg_checksum: Arc<Mutex<Checksum>>) {
        loop {
            let idea_recv_event = match self.idea_recv.try_recv() {
                Ok(event) => event,
                Err(_) => {
                    // If there are no more ideas to receive and a poison pill is received, return early
                    if self.idea_recv.is_empty() && self.is_out_of_ideas_recv.try_recv().is_ok() {
                        return;
                    }
                    continue;
                }
            };

            self.idea = Some(idea_recv_event);
            let pkgs_required = self.idea.as_ref().unwrap().num_pkg_required;

            // loop while self.ideas != none, equivalent of escaping after calling build idea when valid
            loop {
                if pkgs_required <= self.pkgs.len() {
                    self.build_idea(&idea_checksum, &pkg_checksum);
                    break;
                }
                let new_pkg = self.pkg_recv.try_recv();
                if new_pkg.is_ok() {
                    self.pkgs.push(new_pkg.unwrap());
                }
            }
        }
    }
}
