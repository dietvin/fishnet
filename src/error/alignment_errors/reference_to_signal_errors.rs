#[derive(Debug, thiserror::Error)]
pub enum RefToSignalError {
    #[error("No match ops found in Cigar")]
    NoMatchOps,
    #[error("Length of alignment ({0} - 1) discordant with reference length ({1})")]
    DiscordantToSequence(usize, usize),

}