//! Errors, worded for the person using the app.
//!
//! Every message says what happened and what to do next, in plain words
//! (CLAUDE.md). Nothing here ever holds a token, a code or a signed link: the
//! detail strings are built only from status codes, error codes and our own
//! words, never from a server's raw reply.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No stored sign-in.
    NotSignedIn,
    /// The stored sign-in was refused and could not be renewed.
    SignInExpired,
    /// Sign-in did not finish. The text says why.
    SignIn(String),
    /// No connection, or the connection dropped.
    Network(String),
    /// Plaud answered, but with an error. The text says which.
    Plaud(String),
    /// Reading or writing a file or folder failed.
    Disk(String),
    /// The local database could not be read or written.
    Database(String),
    /// The user cancelled.
    Cancelled,
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotSignedIn => write!(f, "You are not signed in to Plaud. Choose Sign in from the File menu or from Settings."),
            Error::SignInExpired => write!(
                f,
                "Your Plaud sign-in has expired. Choose Sign in from the File menu or from Settings to sign in again."
            ),
            Error::SignIn(why) => write!(f, "Sign-in did not finish: {why} Try signing in again."),
            Error::Network(why) => write!(
                f,
                "Could not reach Plaud: {why} Check your internet connection, then try again."
            ),
            Error::Plaud(why) => write!(f, "Plaud reported a problem: {why} Try again later."),
            Error::Disk(why) => write!(f, "{why}"),
            Error::Database(why) => write!(
                f,
                "NoteFetch could not read or write its list of recordings: {why} Close NoteFetch and open it again."
            ),
            Error::Cancelled => write!(f, "Cancelled."),
        }
    }
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        // reqwest's own text can include the URL, which for audio is a signed
        // link. Describe the kind of failure instead.
        let why = if e.is_timeout() {
            "the connection timed out."
        } else if e.is_connect() {
            "the connection could not be made."
        } else if e.is_body() || e.is_decode() {
            "the connection dropped part way through."
        } else {
            "the request failed."
        };
        Error::Network(why.to_string())
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Database(format!("{e}."))
    }
}

/// Describe a file system failure on a path, with what to do next.
pub fn disk(action: &str, path: &std::path::Path, e: &std::io::Error) -> Error {
    use std::io::ErrorKind::*;
    // Windows' "being used by another process" (32) and "locked" (33), as
    // when a player has the audio open. Checked by number, because Rust does
    // not give them a kind of their own on every version.
    let in_use = matches!(e.raw_os_error(), Some(32 | 33));
    let advice = match e.kind() {
        _ if in_use => "A file in it is open in another program, such as a media player. Close it and try again.",
        PermissionDenied => "Windows refused access. If the file is open in another program, such as a media player, close it and try again.",
        NotFound => "It was not found. It may have been moved or deleted outside NoteFetch.",
        StorageFull | QuotaExceeded => "The disk is full. Free some space and try again.",
        AlreadyExists => "Something with that name is already there.",
        _ => "Try again, and if it keeps happening, restart Windows.",
    };
    Error::Disk(format!("Could not {action} \"{}\". {advice}", path.display()))
}
