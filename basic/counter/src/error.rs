use pinocchio::error::ProgramError;

pub enum CounterError {
    InCorrectAccountAddress,
}

impl From<CounterError> for ProgramError {
    fn from(value: CounterError) -> Self {
        match value {
            CounterError::InCorrectAccountAddress => ProgramError::Custom(1),
        }
    }
}
