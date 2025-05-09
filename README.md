# PoliTOcean Purchases Automation Tool
This tools automates the purchase of PoliTOcean products from Mouser and Digikey websites.
## Main Feature
It allows advisors to create, delete and edit orders, allowing them to set Manufacturer Name, Manufacturer Part Number, Quantity,
Proposal and Project fields for each item they want to buy. After this, the tool will automatically look up the products in Mouser and Digikey websites and create an order with the products that are in stock (selecting the cheapest one, when possible). It then creates an excel file with the order details, prices, description and product links automaticatically generated.
## How to use
1. Setup your database connection and APIs, setting up the .env file.
2. Run SQLx migrations.
3. Just cargo run, access the website on "http://yourip:3000"
### .env file setup
The .env file should be located in the root directory of the project. It should contain the following variables:
```
DATABASE_URL=postgres://yourid:yourpwd@yourdbip/yourdbname
MOUSER_API_KEY=yourmouserapikey
DIGIKEY_CLIENT_ID=yourdigikeyclientid
DIGIKEY_CLIENT_SECRET=yourdigikeyclientsecret
```
### SQLx migrations
The SQLx migrations are located in the migrations folder.
To run the migrations, run the following command:
```
DATABASE_URL=postgres://yourid:yourpwd@yourdbip/yourdbname sqlx migrate run
```