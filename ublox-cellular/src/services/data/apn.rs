use heapless::{consts, String};

#[derive(Debug, Clone)]
pub enum Apn {
    Given(String<consts::U99>),
    Automatic,
}

impl Default for Apn {
    fn default() -> Self {
        Apn::Automatic
    }
}

#[derive(Debug, Clone, Default)]
pub struct APNInfo {
    pub apn: Apn,
    pub user_name: Option<String<consts::U64>>,
    pub password: Option<String<consts::U64>>,
}

impl APNInfo {
    pub fn new(apn: &str) -> Self {
        APNInfo {
            apn: Apn::Given(String::from(apn)),
            user_name: None,
            password: None,
        }
    }
}
