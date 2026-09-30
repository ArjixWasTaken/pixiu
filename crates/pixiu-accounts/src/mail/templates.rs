//! The emails píxiū sends, as plain text and simple HTML.

use super::Email;

/// Makes text safe inside HTML.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// An email of a few paragraphs and, perhaps, a button with its link.
fn compose(to: &str, subject: &str, paragraphs: &[String], button: Option<(&str, &str)>) -> Email {
    let mut text = paragraphs.join("\n\n");
    let mut html = String::from(
        "<!doctype html><html><body style=\"margin:0;padding:24px;background:#f6f1ee;\
         font-family:Roboto,Helvetica,Arial,sans-serif;color:#231a15\">\
         <div style=\"max-width:520px;margin:0 auto;background:#ffffff;border-radius:16px;\
         padding:32px\"><p style=\"margin:0 0 24px;font-size:20px\">píxiū</p>",
    );
    for paragraph in paragraphs {
        html.push_str(&format!(
            "<p style=\"margin:0 0 16px;font-size:15px;line-height:1.5\">{}</p>",
            escape(paragraph)
        ));
    }
    if let Some((label, link)) = button {
        text.push_str(&format!("\n\n{label}: {link}"));
        html.push_str(&format!(
            "<p style=\"margin:24px 0\"><a href=\"{link}\" style=\"display:inline-block;\
             padding:12px 24px;border-radius:999px;background:#8f4c2d;color:#ffffff;\
             text-decoration:none;font-size:15px\">{label}</a></p>\
             <p style=\"margin:0;font-size:13px;color:#6f5b50;word-break:break-all\">{link}</p>",
            link = escape(link),
            label = escape(label),
        ));
    }
    html.push_str("</div></body></html>");
    text.push('\n');
    Email {
        to: to.to_owned(),
        subject: subject.to_owned(),
        text,
        html,
    }
}

/// Shows the mail server works.
#[must_use]
pub fn test(to: &str) -> Email {
    compose(
        to,
        "píxiū can send email",
        &["This is a test from píxiū: the mail server works.".to_owned()],
        None,
    )
}

/// Confirms an email address.
#[must_use]
pub fn verify_email(to: &str, username: &str, link: &str) -> Email {
    compose(
        to,
        "Confirm your email address for píxiū",
        &[
            format!("Hi {username},"),
            "Confirm this is your email address, so píxiū can send you password resets and \
             alerts. The link works for 7 days."
                .to_owned(),
        ],
        Some(("Confirm my email", link)),
    )
}

/// Lets someone choose a new password.
#[must_use]
pub fn password_reset(to: &str, username: &str, link: &str) -> Email {
    compose(
        to,
        "Reset your píxiū password",
        &[
            format!("Hi {username},"),
            "Someone asked to reset the password of your píxiū account. If it was you, \
             choose a new one; the link works for an hour. If not, ignore this email: \
             nothing changes."
                .to_owned(),
        ],
        Some(("Choose a new password", link)),
    )
}

/// Tells an admin someone registered.
#[must_use]
pub fn new_registration(to: &str, applicant: &str, applicant_email: &str, link: &str) -> Email {
    compose(
        to,
        &format!("{applicant} asks for a píxiū account"),
        &[
            format!("{applicant} ({applicant_email}) registered on píxiū."),
            "Approve them, and they get an email to confirm their address; deny them, and \
             they get a short note."
                .to_owned(),
        ],
        Some(("Review the request", link)),
    )
}

/// Tells an applicant their registration was declined.
#[must_use]
pub fn registration_declined(to: &str, username: &str) -> Email {
    compose(
        to,
        "Your píxiū registration",
        &[
            format!("Hi {username},"),
            "An admin declined your request for a píxiū account, so none was made.".to_owned(),
        ],
        None,
    )
}

/// Tells the owner of an address someone tried to register with it.
#[must_use]
pub fn email_in_use(to: &str, link: &str) -> Email {
    compose(
        to,
        "Someone registered on píxiū with your email",
        &[
            "Someone asked for a píxiū account with this email address, which already has \
             one. If it was you, sign in, or reset your password if you forgot it. If not, \
             ignore this email."
                .to_owned(),
        ],
        Some(("Sign in", link)),
    )
}

/// Tells a user their YouTube Music session ended.
#[must_use]
pub fn youtube_music_expired(to: &str, username: &str, reason: &str, link: &str) -> Email {
    compose(
        to,
        "Sign in to YouTube Music again on píxiū",
        &[
            format!("Hi {username},"),
            "YouTube Music no longer accepts píxiū's sign-in to your account, so your liked \
             music and private playlists stop syncing. Sign in again to carry on."
                .to_owned(),
            format!("YouTube Music said: {reason}"),
        ],
        Some(("Sign in again", link)),
    )
}

/// Tells a user a watch keeps failing.
#[must_use]
pub fn watch_failing(to: &str, username: &str, watch: &str, error: &str, link: &str) -> Email {
    compose(
        to,
        &format!("píxiū cannot sync “{watch}”"),
        &[
            format!("Hi {username},"),
            format!("Syncing your watch “{watch}” failed several times in a row."),
            format!("The last error: {error}"),
        ],
        Some(("See your watches", link)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emails_escape_what_they_quote() {
        let email = password_reset("a@b.c", "<b>eve</b>", "https://x.test/r?a=1&b=2");
        assert!(email.html.contains("Hi &lt;b&gt;eve&lt;/b&gt;,"));
        assert!(email.html.contains("href=\"https://x.test/r?a=1&amp;b=2\""));
        assert!(email.text.contains("Hi <b>eve</b>,"));
        assert!(
            email
                .text
                .ends_with("Choose a new password: https://x.test/r?a=1&b=2\n")
        );
    }

    #[test]
    fn every_email_reads_right() {
        let link = "https://music.example.com/somewhere";
        insta::assert_snapshot!(
            [
                test("a@example.com"),
                verify_email("a@example.com", "alice", link),
                password_reset("a@example.com", "alice", link),
                new_registration("admin@example.com", "bob", "bob@example.com", link),
                registration_declined("b@example.com", "bob"),
                email_in_use("a@example.com", link),
                youtube_music_expired("a@example.com", "alice", "signed out", link),
                watch_failing("a@example.com", "alice", "Liked music", "timeout", link),
            ]
            .iter()
            .map(|email| format!(
                "To: {}\nSubject: {}\n\n{}",
                email.to, email.subject, email.text
            ))
            .collect::<Vec<_>>()
            .join("\n---\n")
        );
    }
}
