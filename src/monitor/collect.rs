// Copyright 2023 Martin Pool

//! Collect monitored information so that it can be inspected by tests.

#![allow(unused_imports)]

use std::collections::HashSet;
use std::mem::take;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex, Weak};

use crate::Apath;

use super::counters::{Counter, Counters};
use super::task::{Task, TaskInner, TaskList};
use super::{Monitor, Problem};

/// A monitor that collects information for later inspection.
///
/// Problems are collected in a vector.
///
/// Tasks are ignored.
///
/// Totals of counters are kept.
#[derive(Default)]
pub struct CollectMonitor {
    pub problems: Mutex<Vec<Problem>>,
    counters: Counters,
    started_files: Mutex<Vec<Apath>>,
    task_list: Mutex<TaskList>,
}

impl CollectMonitor {
    pub fn new() -> Self {
        CollectMonitor::default()
    }

    pub fn get_counter(&self, counter: Counter) -> usize {
        self.counters.get(counter)
    }

    pub fn take_problems(&self) -> Vec<Problem> {
        take(self.problems.lock().unwrap().as_mut())
    }

    pub fn take_started_files(&self) -> Vec<Apath> {
        take(self.started_files.lock().unwrap().as_mut())
    }
}

impl Monitor for CollectMonitor {
    fn count(&self, counter: Counter, increment: usize) {
        self.counters.count(counter, increment)
    }

    fn set_counter(&self, counter: Counter, value: usize) {
        self.counters.set(counter, value)
    }

    fn problem(&self, problem: Problem) {
        self.problems.lock().unwrap().push(problem);
    }

    fn start_task(&self, name: String) -> Task {
        self.task_list.lock().unwrap().start_task(name)
    }
}
