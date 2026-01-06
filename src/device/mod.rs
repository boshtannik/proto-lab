mod traits;
// mod wired_modem;
mod wireless_modem;

pub use {
    traits::IODriverSimulator,
    /*wired_modem::WiredModemFake,*/ wireless_modem::WirelessModemFake,
};
