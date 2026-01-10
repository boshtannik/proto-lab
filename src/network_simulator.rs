use std::{
    cell::RefCell,
    ops::DerefMut,
    sync::{Arc, Mutex},
};

use crate::EtherSimulator;

pub struct NetworkSimulator {
    ethers: RefCell<Option<Vec<EtherSimulator>>>,
    ms_per_tick: u64,
    simulation_thread_handle: Option<std::thread::JoinHandle<Vec<EtherSimulator>>>,
    thread_killer: Arc<Mutex<bool>>,
}

/// NetworkSimulator is designed to simulate the network which consist of 1+ ethers.
/// Each ether is instance of EtherSimulator
impl NetworkSimulator {
    pub fn new(ms_per_tick: u64) -> Self {
        NetworkSimulator {
            ethers: RefCell::new(Some(Vec::new())),
            ms_per_tick,
            simulation_thread_handle: None,
            thread_killer: Arc::new(Mutex::new(false)),
        }
    }

    pub fn create_ether(&self, name: &str) {
        match self.ethers.borrow_mut().deref_mut() {
            Some(ref mut ethers) => {
                let new_ether = EtherSimulator::new(name);
                ethers.push(new_ether);
            }
            None => {
                panic!("Simulation thread is already started. Can not change configuration")
            }
        };
    }

    pub fn get_ether(&self, name: &str) -> Option<EtherSimulator> {
        match self.ethers.borrow_mut().deref_mut() {
            None => panic!("Simulation thread is started. Can not get ether"),
            Some(ref ethers) => {
                for ether in ethers.iter() {
                    if ether.get_name() == name {
                        return Some(ether.clone());
                    }
                }
                None
            }
        }
    }

    pub fn start_tick(&self) {
        match self.ethers.borrow_mut().deref_mut() {
            None => panic!(
                "Simulation thread is started. Can not do start_tick and thread at the same time"
            ),
            Some(ref ethers) => {
                for ether in ethers.iter() {
                    ether.start_tick();
                }
            }
        }
    }

    pub fn end_tick(&self) {
        match self.ethers.borrow_mut().deref_mut() {
            None => panic!(
                "Simulation thread is started. Can not do start_tick and thread at the same time"
            ),
            Some(ref ethers) => {
                for ether in ethers.iter() {
                    ether.end_tick();
                }
            }
        }
    }

    pub fn simulate(&self) {
        match self.ethers.borrow_mut().deref_mut() {
            None => panic!(
                "Simulation thread is started. Can not do start_tick and thread at the same time"
            ),
            Some(ref mut ethers) => {
                for ether in ethers.iter_mut() {
                    ether.simulate();
                }
            }
        }
    }

    pub fn start_simulation_thread(&mut self) {
        match self.simulation_thread_handle {
            Some(_) => panic!("Simulation thread is already started"),
            None => {
                let mut ethers = self.ethers.take().unwrap();

                let ms_per_tick = self.ms_per_tick;
                let thread_killer_clone = Arc::clone(&self.thread_killer);

                *self
                    .thread_killer
                    .lock()
                    .expect("Fail to get lock on thread killer") = false;

                self.simulation_thread_handle = Some(std::thread::spawn(move || {
                    loop {
                        if *thread_killer_clone
                            .lock()
                            .expect("Faild to get lock on clonned thread killer")
                        {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(ms_per_tick));
                        for ether in ethers.iter_mut() {
                            ether.start_tick();
                        }
                        for ether in ethers.iter_mut() {
                            ether.simulate();
                        }
                        for ether in ethers.iter_mut() {
                            ether.end_tick();
                        }
                    }
                    ethers
                }));
            }
        }
    }

    pub fn stop_simulation_thread(&mut self) {
        self.simulation_thread_handle = match self.simulation_thread_handle.take() {
            None => panic!("Simulation thread is not started"),
            Some(simulation_thread_handle) => {
                *self
                    .thread_killer
                    .lock()
                    .expect("Fail to get lock on thread killer") = true;
                self.ethers.replace(Some(
                    simulation_thread_handle
                        .join()
                        .expect(" Fail to join simulation thread to get ethers back"),
                ));
                None
            }
        };
    }
}
