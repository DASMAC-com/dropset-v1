use solana_sdk::pubkey::Pubkey;
use transaction_parser::ParseDropsetEvents;
use yellowstone_grpc_proto::{
    geyser::{
        subscribe_update::UpdateOneof,
        SubscribeUpdateTransactionInfo,
    },
    prelude::{
        InnerInstruction,
        InnerInstructions,
    },
};

pub struct ParsedInnerInstruction {
    pub parent_index: u32,
    pub program_id: Pubkey,
    pub inner_instruction: InnerInstruction,
}

impl ParseDropsetEvents for ParsedInnerInstruction {
    fn program_id(&self) -> &[u8; 32] {
        self.program_id.as_array()
    }

    fn instruction_data(&self) -> &[u8] {
        &self.inner_instruction.data
    }
}

impl ParsedInnerInstruction {
    fn from_inner_instructions(accounts: &[Pubkey], inner_ixns: InnerInstructions) -> Vec<Self> {
        inner_ixns
            .instructions
            .into_iter()
            .map(|ixn| Self {
                parent_index: inner_ixns.index,
                program_id: accounts[ixn.program_id_index as usize],
                inner_instruction: ixn,
            })
            .collect()
    }
}

pub fn parse_update(update: UpdateOneof) {
    match update {
        UpdateOneof::Account(acc) => {
            if let Some(_account) = acc.account {
                // println!("Account data: {:?}", &account.data[0..100]);
            }
        }
        UpdateOneof::Transaction(update) => {
            if let Some(txn) = update.transaction {
                let account_keys = get_flattened_accounts_in_txn_update(&txn);
                let (logs, parsed_inner_instructions) = if let Some(meta) = txn.meta {
                    meta.compute_units_consumed
                        .inspect(|cu| println!("CU consumed: {}", cu));
                    let logs = meta.log_messages;
                    let parsed_inner_instructions: Vec<ParsedInnerInstruction> = meta
                        .inner_instructions
                        .into_iter()
                        .flat_map(|inner_ixns| {
                            ParsedInnerInstruction::from_inner_instructions(
                                &account_keys,
                                inner_ixns,
                            )
                        })
                        .collect();
                    (logs, parsed_inner_instructions)
                } else {
                    (vec![], vec![])
                };

                if !logs.is_empty() {
                    for log in logs.iter().filter(|s| s.contains("[DEBUG]: ")) {
                        println!("------ LOGS -------");
                        println!("{:?}", log);
                    }
                }
                for inner_ixn in parsed_inner_instructions.iter() {
                    let events = inner_ixn
                        .parse_events()
                        .expect("Should be able to parse events");
                    if !events.is_empty() {
                        println!("----- EVENTS ------");
                        println!("Parent index: {}", inner_ixn.parent_index);
                        println!("{:?}", events);
                    }
                }
            }
        }
        _ => (),
    }
}

fn get_flattened_accounts_in_txn_update(txn: &SubscribeUpdateTransactionInfo) -> Vec<Pubkey> {
    [
        txn.meta.as_ref().map_or(vec![], |meta| {
            [
                meta.loaded_writable_addresses.clone(),
                meta.loaded_readonly_addresses.clone(),
            ]
            .concat()
        }),
        txn.transaction
            .as_ref()
            .and_then(|txn| txn.message.as_ref())
            .map_or(vec![], |msg| msg.account_keys.clone()),
    ]
    .concat()
    .into_iter()
    .filter_map(|vec| Pubkey::try_from(vec).ok())
    .collect::<Vec<Pubkey>>()
}
