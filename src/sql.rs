// SQL statements
#![allow(dead_code)]
pub const CREATE_USER: &str = "INSERT INTO Users (id, username, password) VALUES ($1, $2, $3);";
pub const GET_USER: &str = "SELECT * FROM Users WHERE id = $1";
pub const GET_USER_BYNAME: &str = "SELECT * FROM Users WHERE username = $1";
pub const CREATE_SESSION: &str = "INSERT INTO Sessions (id, userid, expires) VALUES ($1, $2, $3);";
pub const DELETE_SESSION_BYID: &str = "DELETE FROM Sessions WHERE id = $1;";
pub const GET_SESSION_BYID: &str = "SELECT * FROM Sessions WHERE id = $1;";
pub const UPDATE_SESSION_BYID: &str = "UPDATE Sessions SET expires = $1 WHERE id = $2;";