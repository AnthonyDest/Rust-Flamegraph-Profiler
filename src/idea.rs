use super::checksum::Checksum;
use super::Event;
use crossbeam::channel::Sender;
use std::fs;
use std::sync::{Arc, Mutex};

pub struct Idea {
    pub name: String,
    pub num_pkg_required: usize,
}

pub struct IdeaGenerator {
    idea_start_idx: usize,
    num_ideas: usize,
    num_students: usize,
    num_pkgs: usize,
    idea_send: Sender<Idea>,
    is_out_of_ideas_send: Sender<bool>,
    ideas: Arc<Vec<(String, String)>>,
}

impl IdeaGenerator {
    pub fn new(
        idea_start_idx: usize,
        num_ideas: usize,
        num_students: usize,
        num_pkgs: usize,
        idea_send: Sender<Idea>,
        is_out_of_ideas_send: Sender<bool>,
        ideas: Arc<Vec<(String, String)>>,
    ) -> Self {
        Self {
            idea_start_idx,
            num_ideas,
            num_students,
            num_pkgs,
            idea_send,
            is_out_of_ideas_send,
            ideas,
        }
    }

    // Idea names are generated from cross products between product names and customer names
    fn get_next_idea_name(&self, idx: usize) -> String {
        let pair = &self.ideas[idx % self.ideas.len()];
        format!("{} for {}", pair.0, pair.1)
    }

    pub fn run(&self, idea_checksum: Arc<Mutex<Checksum>>) {
        let pkg_per_idea = self.num_pkgs / self.num_ideas;
        let extra_pkgs = self.num_pkgs % self.num_ideas;

        // Generate a set of new ideas and place them into the event-queue
        // Update the idea checksum with all generated idea names

        // compute checksum before acquiring mutex, update is faster than compute
        let mut batch_checksum = Checksum::default();
        for i in 0..self.num_ideas {
            let name = self.get_next_idea_name(self.idea_start_idx + i);
            let extra = (i < extra_pkgs) as usize;
            let num_pkg_required = pkg_per_idea + extra;
            let idea = Idea {
                name,
                num_pkg_required,
            };

            batch_checksum.update(Checksum::with_sha256(&idea.name));

            // send a new idea when recived, no need for event abstraction
            self.idea_send.send(idea).unwrap();
        }
        idea_checksum.lock().unwrap().update(batch_checksum);

        // Push student termination events into the event queue
        for _ in 0..self.num_students {
            // send true when out of ideas
            self.is_out_of_ideas_send.send(true).unwrap();
        }
    }
}
