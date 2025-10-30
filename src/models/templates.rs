use askama::Template;
use crate::{data::order::Order,models::{item::OrderItem, user_info::UserInfo}};

#[derive(Template)]
#[template(path = "pages/new_order.html")]
pub struct NewOrderTemplate {
    pub areas: Vec<String>,
    pub sub_areas: Vec<String>,
    pub proposals: Vec<String>,
    pub projects: Vec<String>
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
    pub areas: Vec<String>,
    pub sub_areas: Vec<String>,
    pub proposals: Vec<String>,
    pub projects: Vec<String>,
}

#[derive(Template)]
#[template(path = "pages/board_homepage.html")]
pub struct BoardHomepageTemplate {
    pub orders: Vec<Order>,
}

#[derive(Template)]
#[template(path = "pages/prof_homepage.html")]
pub struct ProfHomepageTemplate {
    pub orders: Vec<Order>,
}

#[derive(Template)]
#[template(path = "pages/coffee.html")]
pub struct CoffeePageTemplate {
    pub order_id: i32,
}

#[derive(Template)]
#[template(path = "pages/order_arithmetic.html")]
pub struct OrderArithmeticPageTemplate {

}

#[derive(Template)]
#[template(path = "pages/reset_password.html")]
pub struct ResetPasswordPageTemplate {
    pub token: String,
}

#[derive(Template)]
#[template(path = "pages/user_settings.html")]
pub struct UserSettingsPageTemplate {
    pub user_info: UserInfo,
}