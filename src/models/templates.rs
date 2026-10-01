use askama::Template;
use crate::{data::{options::{AreaRow, OptionRow}, order::Order}, models::{item::OrderItem, user_info::UserInfo}};

#[derive(Template)]
#[template(path = "pages/new_order.html")]
pub struct NewOrderTemplate {
    pub areas: Vec<String>,
    pub sub_areas: Vec<String>,
    pub proposals: Vec<String>,
    pub projects: Vec<String>,
    pub is_board: bool,
}

#[derive(Template)]
#[template(path = "pages/login.html")]
pub struct LoginPageTemplate {

}

#[derive(Template)]
#[template(path = "pages/advisors_homepage.html")]
pub struct AdvisorHomepageTemplate {
    pub orders: Vec<Order>,
    pub is_board: bool,
}

#[derive(Template)]
#[template(path = "pages/edit_order.html")]
pub struct EditOrderTemplate {
    pub order: Order,
    pub items: Vec<OrderItem>,
    // Division and sub-area are two dropdowns but form a composite key in `areas`.
    // `area_pairs` lists every valid (division, sub_area) pair so the client can
    // keep the sub-area dropdown constrained to valid, non-archived combinations.
    pub divisions: Vec<String>,
    pub sub_areas: Vec<String>,
    pub area_pairs: Vec<AreaRow>,
    // Proposals/projects carry an `archived` flag so archived options can be
    // rendered disabled: still shown for items that already reference them, but
    // not selectable for new items.
    pub proposals: Vec<OptionRow>,
    pub projects: Vec<OptionRow>,
    pub is_board: bool,
}

/// One line of the "View BOM" page, already formatted for display.
pub struct BomRow {
    pub manufacturer: String,
    pub manufacturer_pn: String,
    pub quantity: i32,
    /// "Mouser" / "Digikey", or empty when the item has no supplier.
    pub provider: &'static str,
    pub supplier_pn: String,
    pub unit_price_vat: String,
    pub total_vat: String,
    /// Why the item has no supplier (only when `provider` is empty).
    pub note: String,
}

#[derive(Template)]
#[template(path = "pages/view_bom.html")]
pub struct ViewBomTemplate {
    pub order: Order,
    pub rows: Vec<BomRow>,
    pub total_vat: String,
    pub is_board: bool,
}

#[derive(Template)]
#[template(path = "pages/board_home.html")]
pub struct BoardHomeTemplate {
    pub is_board: bool,
}

#[derive(Template)]
#[template(path = "pages/board_homepage.html")]
pub struct BoardHomepageTemplate {
    pub orders: Vec<Order>,
    pub is_board: bool,
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
    pub is_board: bool,
}

#[derive(Template)]
#[template(path = "pages/reset_password.html")]
pub struct ResetPasswordPageTemplate {
    pub token: String,
    pub is_board: bool,
}

#[derive(Template)]
#[template(path = "pages/user_settings.html")]
pub struct UserSettingsPageTemplate {
    pub user_info: UserInfo,
    pub is_board: bool,
}

#[derive(Template)]
#[template(path = "pages/manage_users.html")]
pub struct ManageUsersTemplate {
    pub users: Vec<UserInfo>,
    pub divisions: Vec<String>,
    pub sub_areas: Vec<String>,
    // Every (division, sub_area) pair, so the client can constrain the sub-area
    // dropdowns to combinations that actually exist.
    pub area_pairs: Vec<AreaRow>,
    pub roles: Vec<String>,
    pub is_board: bool,
}

#[derive(Template)]
#[template(path = "pages/manage_options.html")]
pub struct ManageOptionsTemplate {
    pub areas: Vec<AreaRow>,
    pub projects: Vec<OptionRow>,
    pub proposals: Vec<OptionRow>,
    pub is_board: bool,
}
