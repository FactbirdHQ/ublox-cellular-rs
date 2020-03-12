pub enum CME {

}


// Idea:

// #[derive(ATATErr)]
// #[at_err("+CME ERROR")]
// pub enum CmeError {
//     #[at_arg(0, "Phone failure")]
//     PhoneFailure,

// }


// impl Display for CmeError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             CmeError::PhoneFailure => write!(f, "Phone failure")
//         }
//     }
// }
