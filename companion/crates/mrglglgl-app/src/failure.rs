//! Why a command failed, by kind, so the window can say it in the player's language and give the
//! next step that fits. The backend's own wording stays only for failures nobody planned for.

use mrglglgl_core::addon_install::InstallError;
use mrglglgl_llm::catalogue::{DownloadError, gigabytes};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[ts(export)]
pub enum Failure {
    /// No answer at all: no connection, or the server could not be reached.
    Offline,
    /// The server answered with an error of its own, or too slowly; it usually passes.
    #[serde(rename_all = "camelCase")]
    ServerTrouble {
        status: Option<u16>,
    },
    /// The release's file failed a check; downloading it again rarely helps.
    #[serde(rename_all = "camelCase")]
    BadPackage {
        detail: String,
    },
    GameRunning,
    /// Sizes in decimal gigabytes, the unit the models list shows.
    #[serde(rename_all = "camelCase")]
    NoSpace {
        needed_gb: String,
        free_gb: String,
    },
    /// The release writes a journal this version of the app cannot read yet.
    AppTooOld,
    /// The player stopped it.
    Cancelled,
    #[serde(rename_all = "camelCase")]
    Other {
        message: String,
    },
}

impl Failure {
    fn other(message: impl ToString) -> Self {
        Failure::Other {
            message: message.to_string(),
        }
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Failure::other(message)
    }
}

impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        Failure::other(message)
    }
}

impl From<InstallError> for Failure {
    fn from(error: InstallError) -> Self {
        match error {
            InstallError::GameRunning => Failure::GameRunning,
            InstallError::NewerSchema(_) => Failure::AppTooOld,
            InstallError::TooLarge
            | InstallError::Checksum
            | InstallError::Unreadable(_)
            | InstallError::BadPath(_)
            | InstallError::BadFile(_)
            | InstallError::NoToc
            | InstallError::WrongInterface(_)
            | InstallError::WrongVersion { .. } => Failure::BadPackage {
                detail: error.to_string(),
            },
            InstallError::NotForever(_)
            | InstallError::NoInterface(_)
            | InstallError::DevelopmentCopy
            | InstallError::NotManaged
            | InstallError::NotInstalled
            | InstallError::Io(_) => Failure::other(error),
        }
    }
}

impl From<DownloadError> for Failure {
    fn from(error: DownloadError) -> Self {
        match error {
            DownloadError::Cancelled => Failure::Cancelled,
            DownloadError::NoSpace { needed, free } => Failure::NoSpace {
                needed_gb: gigabytes(needed),
                free_gb: gigabytes(free),
            },
            // The download reports reqwest's errors as text: a send or a read that broke.
            DownloadError::Network(_) => Failure::Offline,
            DownloadError::Status(status) => from_status(status, error),
            DownloadError::WrongSize { .. } | DownloadError::Io(_) => Failure::other(error),
        }
    }
}

impl From<reqwest::Error> for Failure {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Failure::ServerTrouble { status: None }
        } else if let Some(status) = error.status() {
            from_status(status.as_u16(), error)
        } else if error.is_connect() || error.is_request() || error.is_body() {
            Failure::Offline
        } else {
            Failure::other(error)
        }
    }
}

/// An HTTP status the server sent back: its own trouble (5xx) passes, anything else is named.
pub fn from_status(status: u16, message: impl ToString) -> Failure {
    if (500..600).contains(&status) {
        Failure::ServerTrouble {
            status: Some(status),
        }
    } else {
        Failure::other(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_server_error_or_a_slow_answer_is_server_trouble() {
        assert_eq!(
            from_status(504, "504 Gateway Time-out"),
            Failure::ServerTrouble { status: Some(504) }
        );
        assert_eq!(
            Failure::from(DownloadError::Status(503)),
            Failure::ServerTrouble { status: Some(503) }
        );
    }

    #[test]
    fn a_client_error_keeps_its_wording() {
        assert_eq!(
            from_status(404, "404 Not Found"),
            Failure::Other {
                message: "404 Not Found".into()
            }
        );
    }

    #[test]
    fn a_package_that_fails_a_check_is_a_bad_package() {
        for error in [
            InstallError::BadFile("Mrglglgl/docs-site/astro.config.mjs".into()),
            InstallError::BadPath("../x".into()),
            InstallError::NoToc,
            InstallError::Checksum,
            InstallError::TooLarge,
            InstallError::WrongInterface("110000".into()),
        ] {
            let detail = error.to_string();
            assert_eq!(Failure::from(error), Failure::BadPackage { detail });
        }
    }

    #[test]
    fn install_refusals_with_their_own_advice_keep_their_kind() {
        assert_eq!(
            Failure::from(InstallError::GameRunning),
            Failure::GameRunning
        );
        assert_eq!(
            Failure::from(InstallError::NewerSchema(9)),
            Failure::AppTooOld
        );
    }

    #[test]
    fn a_download_names_the_space_in_gigabytes() {
        assert_eq!(
            Failure::from(DownloadError::NoSpace {
                needed: 3_814_679_712,
                free: 1_250_000_000,
            }),
            Failure::NoSpace {
                needed_gb: "3.81".into(),
                free_gb: "1.25".into(),
            }
        );
    }

    #[test]
    fn a_dropped_connection_or_a_stop_have_their_kind() {
        assert_eq!(
            Failure::from(DownloadError::Network("connection reset".into())),
            Failure::Offline
        );
        assert_eq!(Failure::from(DownloadError::Cancelled), Failure::Cancelled);
    }

    #[test]
    fn the_window_reads_the_kind_from_a_tag() {
        let json = serde_json::to_value(Failure::NoSpace {
            needed_gb: "3.81".into(),
            free_gb: "1.25".into(),
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"kind": "noSpace", "neededGb": "3.81", "freeGb": "1.25"})
        );
    }
}
