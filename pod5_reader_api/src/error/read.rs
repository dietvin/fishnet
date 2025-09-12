/// Enumerates possible errors that can occur when attempting to access required 
/// fields of a Pod5Read. Currently includes a variant for missing or unset fields.
#[derive(Debug, thiserror::Error)]
pub enum Pod5ReadError {
    #[error("Field '{0}' is missing or null")]
    MissingField(&'static str),
}
