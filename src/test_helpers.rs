macro_rules! setup_test_env {
    () => {
        extern crate env_logger;
        extern crate std;

        extern crate nb;

        // Note this useful idiom: importing names from outer (for mod tests) scope.
        use super::*;

        use heapless::{consts::*, spsc::Queue, String};
        #[allow(unused_imports)]
        use log::{error, info, warn};

        use crate::wifi;
        use env_logger::Env;
        use std::sync::Once;

        static INIT: Once = Once::new();

        #[derive(Clone, Copy)]
        struct MilliSeconds(u32);

        trait U32Ext {
            fn s(self) -> MilliSeconds;
            fn ms(self) -> MilliSeconds;
        }

        impl U32Ext for u32 {
            fn s(self) -> MilliSeconds {
                MilliSeconds(self / 1000)
            }
            fn ms(self) -> MilliSeconds {
                MilliSeconds(self)
            }
        }

        struct Timer6;
        impl embedded_hal::timer::CountDown for Timer6 {
            type Time = MilliSeconds;
            fn start<T>(&mut self, _: T)
            where
                T: Into<MilliSeconds>,
            {
            }

            fn wait(&mut self) -> ::nb::Result<(), void::Void> {
                Err(nb::Error::WouldBlock)
            }
        }

        impl embedded_hal::timer::Cancel for Timer6 {
            type Error = ();
            fn cancel(&mut self) -> Result<(), Self::Error> {
                Ok(())
            }
        }
    };
}

macro_rules! setup_test_case {
    () => {{
        INIT.call_once(|| {
            env_logger::Builder::from_env(Env::default().default_filter_or("info"))
                .is_test(true)
                .init();
        });

        static mut WIFI_REQ_Q: Option<Queue<RequestType, U5, u8>> = None;
        static mut WIFI_RES_Q: Option<Queue<Result<ResponseType, at::Error>, U5, u8>> = None;

        unsafe { WIFI_REQ_Q = Some(Queue::u8()) };
        unsafe { WIFI_RES_Q = Some(Queue::u8()) };

        let (wifi_req_p, wifi_req_c) = unsafe { WIFI_REQ_Q.as_mut().unwrap().split() };
        let (wifi_res_p, wifi_res_c) = unsafe { WIFI_RES_Q.as_mut().unwrap().split() };

        let wifi_client = at::client::ATClient::new((wifi_req_p, wifi_res_c), 1000.ms(), Timer6);

        let ublox = UbloxClient::new(wifi_client);

        (ublox, (wifi_req_c, wifi_res_p))
    }};
}

macro_rules! cleanup_test_case {
    ($connection: expr, $req_c: expr) => {
        // let wifi_client = $connection.unwrap().disconnect();
        // let (_, mut wifi_res_c) = wifi_client.release();
        // assert!(wifi_res_c.dequeue().is_none());
        assert!($req_c.dequeue().is_none());
    };
}
