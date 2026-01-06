use std::sync::{Arc, Mutex};

use crate::{device::IODriverSimulator, WirelessModemFake};

pub struct EtherSimulator {
    name: String,
    devices: Arc<Mutex<Vec<WirelessModemFake>>>,
}

impl EtherSimulator {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            devices: Arc::new(Mutex::new(vec![])),
        }
    }

    /// Gets the name of the ether
    /// ```
    /// use proto_lab::EtherSimulator;
    /// let mut ether = EtherSimulator::new("my_ether");
    /// assert_eq!(ether.get_name(), "my_ether");
    /// ```
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Registers a new device (driver / modem)
    /// ```
    /// use proto_lab::EtherSimulator;
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    ///
    /// let mut ether = EtherSimulator::new("my_ether");
    /// ether.register_driver(WirelessModemFake::new("my_modem"));
    /// assert_eq!(ether.get_driver("my_modem").unwrap().get_name(), "my_modem");
    /// ```
    pub fn register_driver(&mut self, driver: WirelessModemFake) {
        let mut devices = self.devices.lock().expect("Fail to get lock on devices");
        devices.push(WirelessModemFake::clone(&driver));
    }

    /// Unregisters a device
    /// ```
    /// use proto_lab::EtherSimulator;
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    ///
    /// let mut ether = EtherSimulator::new("my_ether");
    /// ether.register_driver(WirelessModemFake::new("my_modem"));
    /// ether.unregister_driver("my_modem");
    /// assert!(ether.get_driver("my_modem").is_none());
    /// ```
    pub fn unregister_driver(&mut self, name: &str) {
        let mut devices = self.devices.lock().expect("Fail to get lock on devices");

        loop {
            let mut index_to_remove: Option<_> = None;

            for (i, device) in devices.iter_mut().enumerate() {
                if device.get_name() == name {
                    index_to_remove.replace(i);
                }
            }

            match index_to_remove {
                Some(i) => devices.remove(i),
                None => break,
            };
        }
    }

    /// Gets a registered device
    /// ```
    /// use proto_lab::EtherSimulator;
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    ///
    /// let mut ether = EtherSimulator::new("my_ether");
    /// assert!(ether.get_driver("my_modem").is_none());
    /// ether.register_driver(WirelessModemFake::new("my_modem"));
    /// assert_eq!(ether.get_driver("my_modem").unwrap().get_name(), "my_modem");
    /// ```
    pub fn get_driver(&self, name: &str) -> Option<WirelessModemFake> {
        let devices = self.devices.lock().expect("Fail to get lock on devices");

        for device in devices.iter() {
            if device.get_name() == name {
                return Some(WirelessModemFake::clone(&device));
            }
        }
        None
    }

    /// Gets the broadcasted byte from latest broadasting device.
    /// That is the place where the data collision is possible.
    fn get_current_byte(&self) -> Option<u8> {
        let mut result: Option<u8> = None;
        let devices = self.devices.lock().expect("Fail to get lock on devices");

        for device in devices.iter() {
            if let Some(byte) = device.get_from_device_network_side() {
                result = Some(byte);
            }
        }

        result
    }

    /// Prepares all the registered devices for starting of simulation during tick.
    pub fn start_tick(&self) {
        let devices = self.devices.lock().expect("Fail to get lock on devices");
        for device in devices.iter() {
            device.start_tick();
        }
    }

    /// Prepares all the registered devices for ending of simulation during tick.
    pub fn end_tick(&self) {
        let devices = self.devices.lock().expect("Fail to get lock on devices");
        for device in devices.iter() {
            device.end_tick();
        }
    }

    /// This operation shall be called only during tick is active.
    pub fn simulate(&self) {
        let current_byte = self.get_current_byte();

        let devices = self.devices.lock().expect("Fail to get lock on devices");

        if let Some(current_byte) = current_byte {
            for device in devices.iter() {
                device.put_to_device_network_side(current_byte);
            }
        }
    }

    /// Clones itself.
    /// Also makes all internal data shared to be able to use from multiple threads.
    /// ```
    /// use proto_lab::EtherSimulator;
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    ///
    /// let mut ether = EtherSimulator::new("my_ether");
    /// let ether_clone = ether.clone();
    ///
    /// assert_eq!(ether.get_name(), ether_clone.get_name());
    pub fn clone(&self) -> EtherSimulator {
        EtherSimulator {
            name: String::from(&self.name),
            devices: Arc::clone(&self.devices),
        }
    }
}
