#[derive(Debug, Clone)]
pub enum Apn<'a> {
    Given(&'a str),
    Automatic,
}

impl<'a> Default for Apn<'a> {
    fn default() -> Self {
        Self::Automatic
    }
}

#[derive(Debug, Clone, Default)]
pub struct APNInfo<'a> {
    pub apn: Apn<'a>,
    pub user_name: Option<&'a str>,
    pub password: Option<&'a str>,
}

impl<'a> APNInfo<'a> {
    #[must_use]
    pub fn new(apn: &'a str) -> Self {
        Self {
            apn: Apn::Given(apn),
            user_name: None,
            password: None,
        }
    }
}
