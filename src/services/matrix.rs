#![allow(dead_code)]

use tracing::info;

use crate::{
    error::AppError,
    models::{
        checkin::Checkin,
        settings::{GlobalConfig, UserConfig},
    },
};

pub struct MatrixService;

impl MatrixService {
    pub async fn send_low_mood_notification(
        user_cfg: &UserConfig,
        global_cfg: &GlobalConfig,
        checkin: &Checkin,
    ) -> Result<(), AppError> {
        info!(
            user = %user_cfg.username,
            template = %global_cfg.low_mood_message_template,
            mood = checkin.mood,
            "matrix low mood notification would be sent"
        );
        Ok(())
    }

    pub async fn send_panic_notification(
        user_cfg: &UserConfig,
        global_cfg: &GlobalConfig,
        checkin: Option<&Checkin>,
    ) -> Result<(), AppError> {
        info!(
            user = %user_cfg.username,
            template = %global_cfg.panic_message_template,
            mood = checkin.map(|c| c.mood),
            "matrix panic notification would be sent"
        );
        Ok(())
    }

    pub async fn send_test_message(
        user_cfg: &UserConfig,
        _global_cfg: &GlobalConfig,
    ) -> Result<(), AppError> {
        info!(
            user = %user_cfg.username,
            contact = %user_cfg.matrix_user_id,
            "matrix test notification would be sent"
        );
        Ok(())
    }
}
