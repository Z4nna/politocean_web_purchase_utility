use askama::Template;
use crate::data::{order::Order, item::OrderItem};

#[derive(Template)]
#[template(path = "pages/new_order.html")]
pub struct NewOrderTemplate {

}

#[derive(Template)]
#[template(path = "pages/login.html")]
pub struct LoginPageTemplate {

}

#[derive(Template)]
#[template(path = "pages/advisors_homepage.html")]
pub struct AdvisorHomepageTemplate {
    pub orders: Vec<Order>,
}

#[derive(Template)]
#[template(path = "pages/edit_order.html")]
pub struct EditOrderTemplate {
    pub order: Order,
    pub items: Vec<OrderItem>,
}