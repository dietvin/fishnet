#[derive(Debug, thiserror::Error)]
pub enum QueryToSignalError {
    #[error("Length of alignment ({0}) discordant with query length ({1})")]
    DiscordantToSequence(usize, usize),
    #[error("Length of alignment ({0}) discordant with signal length ({1} / {2} = {3})")]
    DiscordantToSignal(usize, usize, usize, usize)
}