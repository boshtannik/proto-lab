use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use crate::{device::IODriverSimulator, WirelessModemFake};

pub struct EtherSimulator {
    name: String,
    devices: Arc<Mutex<Vec<WirelessModemFake>>>,
    last_broadcasted_device: Option<String>,
}

impl EtherSimulator {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            devices: Arc::new(Mutex::new(vec![])),
            last_broadcasted_device: None,
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

    /// Gets the broadcasted byte from broadasting devices.
    /// Simulates data collections within the ether.
    /// ```
    /// use proto_lab::EtherSimulator;
    /// use proto_lab::IODriverSimulator;
    /// use proto_lab::WirelessModemFake;
    ///
    /// let mut ether = EtherSimulator::new("ether");
    ///
    /// let modem_1 = WirelessModemFake::new("modem_1");
    /// let modem_2 = WirelessModemFake::new("modem_2");
    ///
    /// ether.register_driver(modem_1.clone());
    /// ether.register_driver(modem_2.clone());
    ///
    /// modem_1.put_to_rx_pin(b'a');
    /// modem_1.put_to_rx_pin(b'b');
    ///
    /// ether.start_tick();
    /// ether.simulate();
    /// ether.end_tick();
    ///
    /// assert_eq!(modem_2.get_from_tx_pin().expect("No byte"), b'a');
    ///
    /// ether.start_tick();
    /// ether.simulate();
    /// ether.end_tick();
    ///
    /// assert_eq!(modem_2.get_from_tx_pin().expect("No byte"), b'b');
    /// ```
    fn get_current_byte(&mut self) -> Option<u8> {
        let devices = self.devices.lock().expect("Fail to get lock on devices");
        let mut broadcasted_data: BTreeMap<String, u8> = BTreeMap::new();

        // Collect all broadcasts.
        for device in devices.iter() {
            if let Some(byte) = device.get_from_device_network_side() {
                broadcasted_data
                    .entry(device.get_name().to_owned())
                    .and_modify(|el| *el = byte)
                    .or_insert(byte);
            }
        }

        // In case if amount of broadcast devices is grheather than 1 - filters out
        // data of device which broadcast had registered on the previous iteration
        // of simulation. This technics simulates data collision.
        match self.last_broadcasted_device.take() {
            None => (),
            Some(name_of_last_broadcasted) => {
                if broadcasted_data.len() > 1 {
                    broadcasted_data.retain(|name, _| *name.clone() != name_of_last_broadcasted);
                }
            }
        }

        for (name, byte) in broadcasted_data.iter() {
            self.last_broadcasted_device.replace(name.clone());
            return Some(*byte);
        }

        self.last_broadcasted_device.take();
        return None;
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
    pub fn simulate(&mut self) {
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
            last_broadcasted_device: self.last_broadcasted_device.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_of_collisions() {
        use super::EtherSimulator;
        use super::IODriverSimulator;
        use super::WirelessModemFake;

        let mut ether = EtherSimulator::new("ether");

        let sending_modem_1 = WirelessModemFake::new("modem_1");
        let sending_modem_2 = WirelessModemFake::new("modem_2");
        let receiving_modem = WirelessModemFake::new("modem_3");

        ether.register_driver(sending_modem_1.clone());
        ether.register_driver(sending_modem_2.clone());
        ether.register_driver(receiving_modem.clone());

        let bytes_from_senging_modem_1 = vec![b'a', b'b', b'c', b'd', b'e'];
        let bytes_from_sending_modem_2 = vec![b'f', b'g', b'h', b'i', b'j'];

        for b in bytes_from_senging_modem_1.iter() {
            sending_modem_1.put_to_rx_pin(*b);
        }
        for b in bytes_from_sending_modem_2.iter() {
            sending_modem_2.put_to_rx_pin(*b);
        }

        let mut num_caught_from_modem_1: usize = 0;
        let mut num_caught_from_modem_2: usize = 0;
        let mut total_bytes_received: usize = 0;

        ether.start_tick();
        ether.simulate();
        ether.end_tick();
        while let Some(got_byte) = receiving_modem.get_from_tx_pin() {
            total_bytes_received += 1;
            if bytes_from_senging_modem_1.contains(&got_byte) {
                num_caught_from_modem_1 += 1;
            } else if bytes_from_sending_modem_2.contains(&got_byte) {
                num_caught_from_modem_2 += 1;
            } else {
                panic!("Unexpected scenario. Caught byte which has not been sent");
            }
            ether.start_tick();
            ether.simulate();
            ether.end_tick();
        }

        assert!(num_caught_from_modem_1 > 0);
        assert!(num_caught_from_modem_1 < 5);

        assert!(num_caught_from_modem_2 > 0);
        assert!(num_caught_from_modem_2 < 5);

        assert!(total_bytes_received == 5);
    }
}
