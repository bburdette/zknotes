use crate::util;
use lettre::smtp::response::Response;
use lettre::{EmailAddress, Envelope, SendableEmail, SmtpClient, SmtpTransport, Transport};
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
  let email = SendableEmail::new(
    Envelope::new(
      Some(EmailAddress::new("no-reply@zknotes.com".to_string())?),
      vec![EmailAddress::new(email.to_string())?],
    )?,
    "change zknotes email".to_string(),
    (format!(
      "Click the link to change to your new email, {} user '{}'!  \
       {}/newemail/{}/{}",
      appname, uid, mainsite, uid, newemail_token
    ))
    .to_string()
    .into_bytes(),
  );

  // to help with registration for desktop use, or if the server is barred from sending email.
  util::write_string(
    "last-email-change.txt",
    (format!(
      "Click the link to change to your new email, {} user '{}'!  \
       {}/newemail/{}/{}",
      appname, uid, mainsite, uid, newemail_token
    ))
    .to_string()
    .as_str(),
  )?;

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email).map_err(|e| e.into())
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
  let email = SendableEmail::new(
    Envelope::new(
      Some(EmailAddress::new("no-reply@zknotes.com".to_string())?),
      vec![EmailAddress::new(email.to_string())?],
    )?,
    "zknotes registration".to_string(),
    (format!(
      "Click the link to complete registration, {} user '{}'!  \
       {}/register/{}/{}",
      appname, uid, mainsite, uid, reg_id
    ))
    .to_string()
    .into_bytes(),
  );

  // to help with registration for desktop use, or if the server is barred from sending email.
  util::write_string(
    "last-email.txt",
    (format!(
      "Click the link to complete registration, {} user '{}'!  \
       {}/register/{}/{}",
      appname, uid, domain, uid, reg_id
    ))
    .to_string()
    .as_str(),
  )?;

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email).map_err(|e| e.into())
}

pub fn send_registration_notification(
  appname: &str,
  domain: &str,
  adminemail: &str,
  email: &str,
  uid: &str,
  reg_id: &str,
) -> Result<Response, Box<dyn Error>> {
  info!("sending registration notification to admin!");
  let email = SendableEmail::new(
    Envelope::new(
      Some(EmailAddress::new(
        format!("no-reply@{}", domain).to_string(),
      )?),
      vec![EmailAddress::new(adminemail.to_string())?],
    )?,
    format!(
      "Someones trying to register for {}! {}, {}",
      appname, uid, email
    ),
    format!("uid: {}\nemail:{}\nreg_id: {}", uid, email, reg_id)
      .to_string()
      .into_bytes(),
  );

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email).map_err(|e| e.into())
}
