pub mod parse;

use parse::parsing_error::ParsingError;

const PROGRAM_ID_IDENTIFIER: &str = "program_id";
const ACCOUNT_IDENTIFIER: &str = "account";
const ACCOUNT_NAME: &str = "name";
const ACCOUNT_WRITABLE: &str = "writable";
const ACCOUNT_SIGNER: &str = "signer";
const ARGUMENT_IDENTIFIER: &str = "args";
const DESCRIPTION: &str = "desc";
