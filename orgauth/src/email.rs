use crate::util;
use lettre::smtp::response::Response;
use lettre::{SmtpClient, SmtpTransport, Transport};
use lettre_email::EmailBuilder;
use log::info;
use std::error::Error;

pub fn send_newemail_confirmation(
  appname: &str,
  domain: &str,
  mainsite: &str,
  email: &str,
  uid: &str,
  newemail_token: &str,
) -> Result<Response, Box<dyn Error>> {
  info!("Sending email change confirmation for user: {}", uid);
  let email = EmailBuilder::new()
    .from(format!("no-reply@{}", domain).to_string())
    .to(email)
    .subject(format!("change {} email", appname).to_string())
    .text(
      (format!(
        "Click the link to change to your new email, {} user '{}'!\n\
       {}/newemail/{}/{}",
        appname, uid, mainsite, uid, newemail_token
      ))
      .as_str(),
    )
    .build()?;

  // to help with registration for desktop use, or if the server is barred from sending email.
  util::write_string(
    "last-email-change.txt",
    (format!(
      "Click the link to change to your new email, {} user '{}'!\n\
       {}/newemail/{}/{}",
      appname, uid, mainsite, uid, newemail_token
    ))
    .to_string()
    .as_str(),
  )?;

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email.into()).map_err(|e| e.into())
}

pub fn send_registration(
  appname: &str,
  domain: &str,
  mainsite: &str,
  email: &str,
  uid: &str,
  reg_id: &str,
) -> Result<Response, Box<dyn Error>> {
  info!("Sending registration email for user: {}", uid);
  let email = EmailBuilder::new()
    .from(format!("no-reply@{}", domain).to_string())
    .to(email)
    .subject(format!("{} registration", appname).to_string())
    .text(
      (format!(
        "Click the link to complete registration, {} user '{}'!\n\
       {}/register/{}/{}",
        appname, uid, mainsite, uid, reg_id
      ))
      .as_str(),
    )
    .build()?;

  // to help with registration for desktop use, or if the server is barred from sending email.
  util::write_string(
    "last-email.txt",
    (format!(
      "Click the link to complete registration, {} user '{}'!\n\
       {}/register/{}/{}",
      appname, uid, mainsite, uid, reg_id
    ))
    .to_string()
    .as_str(),
  )?;

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email.into()).map_err(|e| e.into())
}

pub fn send_reset(
  appname: &str,
  domain: &str,
  mainsite: &str,
  email: &str,
  username: &str,
  reset_id: &str,
) -> Result<Response, Box<dyn Error>> {
  info!("Sending reset email for user: {}", username);

  let email = EmailBuilder::new()
    .from(format!("no-reply@{}", domain).to_string())
    .to(email)
    .subject(format!("{} password reset", appname).to_string())
    .text(
      (format!(
        "Click the link to complete password reset, {} user '{}'!\n\
       {}/reset/{}/{}",
        appname, username, mainsite, username, reset_id
      ))
      .as_str(),
    )
    .build()?;

  // to help with reset for desktop use, or if the server is barred from sending email.
  util::write_string(
    "last-email.txt",
    (format!(
      "Click the link to complete reset, {} user '{}'!\n\
       {}/reset/{}/{}",
      appname, username, mainsite, username, reset_id
    ))
    .to_string()
    .as_str(),
  )?;

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email.into()).map_err(|e| e.into())
}

pub fn send_registration_notification(
  appname: &str,
  domain: &str,
  adminemail: &str,
  email: &str,
  uid: &str,
  _reg_id: &str,
) -> Result<Response, Box<dyn Error>> {
  info!("sending registration notification to admin!");
  let email = EmailBuilder::new()
    .from(format!("no-reply@{}", domain).to_string())
    .to(adminemail)
    .subject(format!("{} new registration, uid: {}", appname, uid).to_string())
    .text(
      (format!(
        "Someones trying to register for {}! {}, {}",
        appname, uid, email
      ))
      .as_str(),
    )
    .build()?;

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email.into()).map_err(|e| e.into())
}
