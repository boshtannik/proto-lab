use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use super::IODriverSimulator;

enum AntennaState {
    Transmit(u8),
    Receive(u8),
    Idle,
}

//  Diagram of a half-duplex device, probably radio driver, or single wired transceiver
//  Is made to picture the idea of internal quques connectivities.
//
//```
//                (Network side)
//                       \|/
//                        |  - Antenna
//                        |
//   +--------------------|-----------------+
//   | Wireless Device  +-+-+               |
//   |                  \   \               |
//   |        +------>--+   +-->--+         |
//   |        |                   |         |
//   |        +-<-  [to_ether] -<---+       |
//   |                            | |       |
//   |                            | |       |
//   |     ---<--- [from_ether] <-+ |       |
//   |     |                        |       |
//   |     |                        |       |
//   |     |                        |       |
//   |   TX pin                 RX pin      |
//   +--------------------------------------+
//        |                     |
//        |      (Pins side)    |
//        |                     |
//        o                     o
//```
//
enum TickState {
    InTick,
    OffTick,
}

struct InternalState {
    tick_state: TickState,
    from_antenna_buffer: VecDeque<u8>,
    to_antenna_buffer: VecDeque<u8>,
    antennta_state: AntennaState,
}

impl embedded_io::ErrorType for WirelessModemFake {
    type Error = core::convert::Infallible;
}

impl embedded_io::ReadReady for WirelessModemFake {
    fn read_ready(&mut self) -> Result<bool, Self::Error> {
        Ok(self.readable())
    }
}

impl embedded_io::Read for WirelessModemFake {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        WirelessModemFake::read(&self, buf)
    }
}

impl embedded_io::Write for WirelessModemFake {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        WirelessModemFake::write(&self, buf)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        WirelessModemFake::flush(&self)
    }
}

pub struct WirelessModemFake {
    arc_mutexed_internal_state: Arc<Mutex<InternalState>>,
    name: String,
}

impl WirelessModemFake {
    pub fn new(name: &str) -> Self {
        WirelessModemFake {
            arc_mutexed_internal_state: Arc::new(Mutex::new(InternalState {
                tick_state: TickState::OffTick,
                from_antenna_buffer: VecDeque::new(),
                to_antenna_buffer: VecDeque::new(),
                antennta_state: AntennaState::Idle,
            })),
            name: String::from(name),
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, core::convert::Infallible> {
        let mut count_red: usize = 0;
        for buf_vancant_place in buf.iter_mut() {
            if let Some(byte) = self.get_from_tx_pin() {
                *buf_vancant_place = byte;
                count_red += 1;
            }
        }
        Ok(count_red)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize, core::convert::Infallible> {
        let mut count_written: usize = 0;
        for b in buf {
            self.put_to_rx_pin(*b);
            count_written += 1;
        }
        Ok(count_written)
    }

    pub fn flush(&self) -> Result<(), core::convert::Infallible> {
        Ok(())
    }

    /// While clonning - method internally shares data for all clonned
    /// instances of the modem. So all of them can be used in different
    /// parts of the program, and even in different threads.
    pub fn clone(&self) -> Self {
        WirelessModemFake {
            arc_mutexed_internal_state: Arc::clone(&self.arc_mutexed_internal_state),
            name: self.name.clone(),
        }
    }
}

impl IODriverSimulator for WirelessModemFake {
    /// Simulates that the modem emits a byte towards the ether
    /// ```
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    ///
    /// let device = WirelessModemFake::new("my_modem");
    /// device.start_tick();
    /// assert_eq!(device.get_from_device_network_side(), None);
    /// device.end_tick();
    ///
    /// device.put_to_rx_pin(1);
    ///
    /// device.start_tick();
    /// assert_eq!(device.get_from_device_network_side(), Some(1));
    /// device.end_tick();
    /// ```

    fn get_from_device_network_side(&self) -> Option<u8> {
        let locked_internal_state = self
            .arc_mutexed_internal_state
            .lock()
            .expect(format!("Fail to lock mutex for modem :{}", self.name).as_str());

        match locked_internal_state.tick_state {
            TickState::OffTick => panic!("Impossible to put_to_device_network_side. Device not in simulation mode. Simulation is within the tick. You shall start tick first."),
            TickState::InTick => match locked_internal_state.antennta_state {
                AntennaState::Transmit(byte) => Some(byte),
                _ => None,
            },
        }
    }

    /// Simulates that the modem caught a byte from the ether
    /// ```
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    ///
    /// let device = WirelessModemFake::new("my_modem");
    /// assert_eq!(device.get_from_tx_pin(), None);
    /// device.start_tick();
    /// device.put_to_device_network_side(1);
    /// device.end_tick();
    /// assert_eq!(device.get_from_tx_pin(), Some(1));
    /// ```
    fn put_to_device_network_side(&self, byte: u8) {
        let mut locked_internal_state = self
            .arc_mutexed_internal_state
            .lock()
            .expect(format!("Fail to lock mutex for modem :{}", self.name).as_str());

        match locked_internal_state.tick_state {
            TickState::OffTick => panic!("Impossible to put_to_device_network_side. Device not in simulation mode. Simulation is within the tick. You shall start tick first."),
            TickState::InTick => match locked_internal_state.antennta_state {
                AntennaState::Transmit(_) => (),
                AntennaState::Idle | AntennaState::Receive(_) => {
                    locked_internal_state.antennta_state = AntennaState::Receive(byte)
                }
            },
        }
    }

    /// Reads a byte on the TX pin
    /// ```
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    ///
    /// let device = WirelessModemFake::new("my_modem");
    /// assert_eq!(device.get_from_tx_pin(), None);
    /// device.start_tick();
    /// device.put_to_device_network_side(1);
    /// device.end_tick();
    /// assert_eq!(device.get_from_tx_pin(), Some(1));
    /// ```
    fn get_from_tx_pin(&self) -> Option<u8> {
        let mut locked_internal_state = self
            .arc_mutexed_internal_state
            .lock()
            .expect(format!("Fail to lock mutex for modem :{}", self.name).as_str());

        locked_internal_state.from_antenna_buffer.pop_front()
    }

    /// Writes a byte on the RX pin
    /// ```
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    ///
    /// let device = WirelessModemFake::new("my_modem");
    /// device.put_to_rx_pin(1);
    /// device.start_tick();
    /// assert_eq!(device.get_from_device_network_side(), Some(1));
    /// device.end_tick();
    fn put_to_rx_pin(&self, byte: u8) {
        let mut locked_internal_state = self
            .arc_mutexed_internal_state
            .lock()
            .expect(format!("Fail to lock mutex for modem :{}", self.name).as_str());

        locked_internal_state.to_antenna_buffer.push_back(byte);
    }

    /// Tick is needed only for simulating time during which ineraction with the ether is going.
    /// Other operations like put to pin or get from pin can be done not in tick.
    fn start_tick(&self) {
        let mut locked_internal_state = self
            .arc_mutexed_internal_state
            .lock()
            .expect(format!("Fail to lock mutex for modem :{}", self.name).as_str());

        match locked_internal_state.tick_state {
            TickState::OffTick => {
                locked_internal_state.antennta_state =
                    match locked_internal_state.to_antenna_buffer.pop_front() {
                        Some(byte) => AntennaState::Transmit(byte),
                        _ => AntennaState::Idle,
                    };

                locked_internal_state.tick_state = TickState::InTick;
            }
            TickState::InTick => (),
        }
    }

    /// Tick is needed only for simulating time during which ineraction with the ether is going.
    /// Other operations like put to pin or get from pin can be done not in tick.
    fn end_tick(&self) {
        let mut locked_internal_state = self
            .arc_mutexed_internal_state
            .lock()
            .expect(format!("Fail to lock mutex for modem :{}", self.name).as_str());

        match locked_internal_state.tick_state {
            TickState::OffTick => (),
            TickState::InTick => {
                match locked_internal_state.antennta_state {
                    AntennaState::Receive(byte) => {
                        locked_internal_state.from_antenna_buffer.push_back(byte);
                    }
                    _ => (),
                }

                locked_internal_state.antennta_state = AntennaState::Idle;

                locked_internal_state.tick_state = TickState::OffTick;
            }
        }
    }

    /// Tells if the device has some bytes to be red from pin
    /// ```
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    /// let mut device = WirelessModemFake::new("");
    /// assert!(
    /// !device.readable());
    /// device.start_tick();
    /// device.put_to_device_network_side(1);
    /// device.end_tick();
    /// assert!(device.readable());
    /// ```
    fn readable(&self) -> bool {
        let locked_internal_state = self
            .arc_mutexed_internal_state
            .lock()
            .expect(format!("Fail to lock mutex for modem :{}", self.name).as_str());

        !locked_internal_state.from_antenna_buffer.is_empty()
    }

    /// Tells if the device is ready to be written in
    /// ```
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    /// assert!(WirelessModemFake::new("").writable());
    /// ```
    fn writable(&self) -> bool {
        true
    }

    /// Returns the name of the device
    /// ```
    /// use proto_lab::WirelessModemFake;
    /// use proto_lab::IODriverSimulator;
    /// let device = WirelessModemFake::new("my_modem");
    /// assert_eq!(device.get_name(), "my_modem");
    /// ```
    fn get_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod radio_modem_device_tests {
    use super::*;

    #[test]
    fn test_half_duplex_send_per_tick() {
        let modem_device = WirelessModemFake::new("");
        modem_device.start_tick();
        modem_device.put_to_device_network_side(b'a');
        modem_device.put_to_rx_pin(b'b');
        modem_device.end_tick();

        let byte_on_tx_pin = modem_device.get_from_tx_pin();

        modem_device.start_tick();
        assert_eq!(modem_device.get_from_device_network_side(), Some(b'b'));
        modem_device.end_tick();
        assert_eq!(byte_on_tx_pin, Some(b'a'));
    }

    // Test data collision with overwriting data per same tick
    #[test]
    fn test_data_collision_per_tick() {
        let modem_device = WirelessModemFake::new("");
        modem_device.start_tick();
        modem_device.put_to_device_network_side(b'a');
        modem_device.put_to_device_network_side(b'b');
        modem_device.put_to_device_network_side(b'c');
        modem_device.end_tick();
        assert_eq!(modem_device.get_from_tx_pin(), Some(b'c'));
    }
}
