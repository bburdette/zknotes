use crate::util;
use lettre::smtp::response::Response;
use lettre::{
  EmailAddress, Envelope, Message, SendableEmail, SmtpClient, SmtpTransport, Transport,
};
use lettre_email::EmailBuilder;
use log::info;
use std::error::Error;

pub fn send_newemail_confirmation(
  appname: &str,
  _domain: &str,
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
      "Click the link to change to your new email, {} user '{}'!\n\
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
      "Click the link to change to your new email, {} user '{}'!\n\
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
  _domain: &str,
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
      "Click the link to complete registration, {} user '{}'!\n\
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
      "Click the link to complete registration, {} user '{}'!\n\
       {}/register/{}/{}",
      appname, uid, mainsite, uid, reg_id
    ))
    .to_string()
    .as_str(),
  )?;

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email).map_err(|e| e.into())
}

pub fn send_reset(
  appname: &str,
  _domain: &str,
  mainsite: &str,
  email: &str,
  username: &str,
  reset_id: &str,
) -> Result<Response, Box<dyn Error>> {
  info!("Sending reset email for user: {}", username);
  // let email = SendableEmail::new(
  //   Envelope::new(
  //     Some(EmailAddress::new("no-reply@zknotes.com".to_string())?),
  //     vec![EmailAddress::new(email.to_string())?],
  //   )?,
  //   "zknotes password reset".to_string(),
  //   (format!(
  //     "Click the link to complete password reset, {} user '{}'!\n\
  //      {}/reset/{}/{}",
  //     appname, username, mainsite, username, reset_id
  //   ))
  //   .to_string()
  //   .into_bytes(),
  // );

  let email = EmailBuilder::new()
    .from("no-reply@zknotes.com")
    .reply_to(email)
    .to("Hei <hei@domain.tld>")
    .subject("zknotes password reset")
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

  // let mut mailer = SmtpTransport::builder_unencrypted_localhost()
  //   .unwrap()
  //   .build();

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email.into()).map_err(|e| e.into())
}

fn test() {
  let email = EmailBuilder::new()
    // Addresses can be specified by the tuple (email, alias)
    .to(("user@example.org", "Firstname Lastname"))
    // ... or by an address only
    .from("user@example.com")
    .subject("Hi, Hello world")
    .text("Hello world.")
    .build()
    .unwrap();

  // Open a local connection on port 25
  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost().unwrap());

  // Send the email
  let result = mailer.send(email.into());

  if result.is_ok() {
    println!("Email sent");
  } else {
    println!("Could not send email: {:?}", result);
  }

  assert!(result.is_ok());
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
