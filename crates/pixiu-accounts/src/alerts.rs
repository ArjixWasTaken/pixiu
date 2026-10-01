//! Alerts by email: each user hears about what concerns them, once per
//! event, if they have a confirmed address and want that kind.

use std::{collections::HashMap, sync::Arc};

use pixiu_core::alerts::{Alert, AlertKind, AlertSink};
use pixiu_db::{Db, SentAlert, User, UserSetting, UserStatus, now, toasty};

use crate::mail::{Mailer, templates};

/// The user setting holding which alerts they want.
const ALERTS: &str = "alerts";

/// Which alerts `user_id` wants: every kind, unless they switched it off.
///
/// # Errors
///
/// Fails on database errors.
pub async fn wanted(db: &mut Db, user_id: u64) -> Result<HashMap<AlertKind, bool>, toasty::Error> {
    let stored: HashMap<String, bool> = UserSetting::filter_by_user_id_and_key(user_id, ALERTS)
        .first()
        .exec(db)
        .await?
        .and_then(|setting| serde_json::from_str(&setting.value).ok())
        .unwrap_or_default();
    Ok(AlertKind::ALL
        .into_iter()
        .map(|kind| (kind, stored.get(kind.name()).copied().unwrap_or(true)))
        .collect())
}

/// Switches kinds of alert on or off for `user_id`; kinds left out keep
/// their setting.
///
/// # Errors
///
/// Fails on database errors.
pub async fn set_wanted(
    db: &mut Db,
    user_id: u64,
    changes: &HashMap<AlertKind, bool>,
) -> Result<(), toasty::Error> {
    let mut current = wanted(db, user_id).await?;
    current.extend(changes);
    let value = serde_json::to_string(
        &current
            .iter()
            .map(|(kind, on)| (kind.name(), *on))
            .collect::<HashMap<_, _>>(),
    )
    .expect("settings serialize");
    match UserSetting::filter_by_user_id_and_key(user_id, ALERTS)
        .first()
        .exec(db)
        .await?
    {
        Some(mut setting) => toasty::update!(setting { value }).exec(db).await,
        None => toasty::create!(UserSetting {
            user_id,
            key: ALERTS,
            value,
        })
        .exec(db)
        .await
        .map(|_| ()),
    }
}

/// Emails alerts.
pub struct MailAlerts {
    pub db: Db,
    pub mailer: Arc<Mailer>,
}

impl MailAlerts {
    async fn deliver(&self, user_id: u64, alert: Alert) -> Result<(), toasty::Error> {
        let mut db = self.db.clone();
        let settings = self.mailer.settings().get();
        if !settings.mail_ready() {
            return Ok(());
        }
        let Some(user) = User::filter_by_id(user_id).first().exec(&mut db).await? else {
            return Ok(());
        };
        let (Some(email), Some(_)) = (&user.email, user.email_verified_at) else {
            return Ok(());
        };
        if user.status != UserStatus::Active || !wanted(&mut db, user_id).await?[&alert.kind()] {
            return Ok(());
        }
        let dedupe_key = alert.dedupe_key();
        if SentAlert::filter_by_user_id_and_dedupe_key(user_id, &dedupe_key)
            .first()
            .exec(&mut db)
            .await?
            .is_some()
        {
            return Ok(());
        }
        toasty::create!(SentAlert {
            user_id,
            dedupe_key,
            sent_at: now(),
        })
        .exec(&mut db)
        .await?;
        let message = match &alert {
            Alert::YouTubeMusicExpired { reason, .. } => templates::youtube_music_expired(
                email,
                &user.username,
                reason,
                &settings
                    .link("/settings?tab=youtube-music")
                    .unwrap_or_default(),
            ),
            Alert::WatchFailing { name, error, .. } => templates::watch_failing(
                email,
                &user.username,
                name,
                error,
                &settings.link("/watches").unwrap_or_default(),
            ),
        };
        self.mailer.queue(message);
        tracing::info!(user = user_id, kind = alert.kind().name(), "alert emailed");
        Ok(())
    }
}

impl AlertSink for MailAlerts {
    fn alert(
        &self,
        user: u64,
        alert: Alert,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            if let Err(error) = self.deliver(user, alert).await {
                tracing::error!(%error, user, "cannot send an alert");
            }
        })
    }
}
