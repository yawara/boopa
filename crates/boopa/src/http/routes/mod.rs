use actix_web::web;

pub mod autoinstall;
pub mod cache;
pub mod dhcp;
pub mod distros;
pub mod health;

pub fn configure(cfg: &mut web::ServiceConfig) {
    health::configure(cfg);
    distros::configure(cfg);
    dhcp::configure(cfg);
    cache::configure(cfg);
    autoinstall::configure(cfg);
}
