use lettre::smtp::response::Response;
use lettre::{EmailAddress, Envelope, SendableEmail, SmtpClient, SmtpTransport, Transport};
use std::error::Error;

pub fn send_registration(email: &str, uid: &str, reg_id: &str) -> Result<Response, Box<dyn Error>> {
  info!("Sending registration email for user: {}", uid);
  let email = SendableEmail::new(
    Envelope::new(
      Some(EmailAddress::new("no-reply@practica.site".to_string())?),
      vec![EmailAddress::new(email.to_string())?],
    )?,
    "Practica registration".to_string(),
    (format!(
      "Hey click on this crazy link, practica user '{}'!  \
       https://www.practica.site/register/{}/{}",
      uid, uid, reg_id
    ))
    .to_string()
    .into_bytes(),
  );

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email).map_err(|e| e.into())
}

pub fn send_registration_notification(
  adminemail: &str,
  email: &str,
  uid: &str,
  reg_id: &str,
) -> Result<Response, Box<dyn Error>> {
  info!("sending registration notification to admin!");
  let email = SendableEmail::new(
    Envelope::new(
      Some(EmailAddress::new("no-reply@practica.site".to_string())?),
      vec![EmailAddress::new(adminemail.to_string())?],
    )?,
    format!("Someones trying to register! {}, {}", uid, email),
    format!("uid: {}\nemail:{}\nreg_id: {}", uid, email, reg_id)
      .to_string()
      .into_bytes(),
  );

  let mut mailer = SmtpTransport::new(SmtpClient::new_unencrypted_localhost()?);
  // Send the email
  mailer.send(email).map_err(|e| e.into())
}
