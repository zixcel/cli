use crate::{Error, Result};
use rustix::termios::{self, LocalModes, OptionalActions, SpecialCodeIndex, Termios};
use std::io::{IsTerminal, Read, Write};
use zeroize::Zeroizing;

pub(super) fn require_owner_terminal() -> Result<()> {
    if !std::io::stdin().is_terminal()
        || !std::io::stderr().is_terminal()
        || std::env::var_os("GITHUB_ACTIONS").is_some()
    {
        return Err(Error("interactive_authorization_required"));
    }
    Ok(())
}
pub(super) fn confirm(description: &str, assume_yes: bool) -> Result<()> {
    eprintln!("{description}");
    crate::confirmation::ask(assume_yes)
}
struct Restore(Termios);
impl Drop for Restore {
    fn drop(&mut self) {
        let _ = termios::tcsetattr(std::io::stdin(), OptionalActions::Flush, &self.0);
    }
}
pub(super) fn read_secret() -> Result<Zeroizing<Vec<u8>>> {
    require_owner_terminal()?;
    let original =
        termios::tcgetattr(std::io::stdin()).map_err(|_| Error("credential_input_unavailable"))?;
    let mut hidden = original.clone();
    hidden
        .local_modes
        .remove(LocalModes::ECHO | LocalModes::ECHONL | LocalModes::ICANON | LocalModes::ISIG);
    hidden.special_codes[SpecialCodeIndex::VMIN] = 1;
    hidden.special_codes[SpecialCodeIndex::VTIME] = 0;
    let restore = Restore(original);
    termios::tcsetattr(std::io::stdin(), OptionalActions::Flush, &hidden)
        .map_err(|_| Error("credential_input_unavailable"))?;
    eprint!("Secret (hidden; Ctrl-C cancels): ");
    std::io::stderr()
        .flush()
        .map_err(|_| Error("credential_input_unavailable"))?;
    let mut secret = Zeroizing::new(Vec::with_capacity(16384));
    loop {
        let mut byte = [0];
        std::io::stdin()
            .read_exact(&mut byte)
            .map_err(|_| Error("credential_input_unavailable"))?;
        match byte[0] {
            b'\r' | b'\n' => break,
            3 | 4 => return Err(Error("approval_declined")),
            8 | 127 => {
                secret.pop();
            }
            b if b.is_ascii_graphic() || b == b' ' => {
                if secret.len() == 16384 {
                    return Err(Error("credential_too_large"));
                }
                secret.push(b);
            }
            _ => return Err(Error("invalid_credential_environment")),
        }
    }
    drop(restore);
    eprintln!();
    if secret.is_empty() {
        return Err(Error("invalid_credential_environment"));
    }
    Ok(secret)
}
