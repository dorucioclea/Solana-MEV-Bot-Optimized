//Taken here: https://github.com/MeteoraAg/dlmm-sdk/blob/main/cli/src/instructions/swap.rs

use std::ops::Deref;
use std::rc::Rc;

use anchor_client::solana_client::rpc_config::RpcSendTransactionConfig;

use anchor_client::solana_sdk::compute_budget::ComputeBudgetInstruction;
use anchor_client::Client;
use anchor_client::{solana_sdk::pubkey::Pubkey, solana_sdk::signer::Signer, Program};
use anchor_lang::solana_program::instruction::AccountMeta;
use anchor_spl::associated_token::get_associated_token_address;

// use anyhow::*;
use lb_clmm::accounts;
use lb_clmm::state::bin::BinArray;
use lb_clmm::instruction;
use lb_clmm::state::lb_pair::LbPair;
use lb_clmm::utils::pda::{derive_bin_array_bitmap_extension, derive_bin_array_pda};
use log::info;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Keypair;

// use lb_clmm::state::bin::BinArray;
// use lb_clmm::utils::pda::*;

#[derive(Debug)]
pub struct SwapParameters {
    pub lb_pair: Pubkey,
    pub amount_in: u64,
    pub swap_for_y: bool,
}
// 
pub async fn construct_meteora_instructions(transaction_config: RpcSendTransactionConfig, payer: Keypair, params: SwapParameters, min_amount_out: u64) -> (Instruction, lb_clmm::accounts::Swap, Vec<AccountMeta>, lb_clmm::instruction::Swap) {
    let SwapParameters {
        amount_in,
        lb_pair,
        swap_for_y,
    } = params;

    let commitment_config = CommitmentConfig::confirmed();
    let client = Client::new_with_options(
        anchor_client::Cluster::Mainnet,
        Rc::new(Keypair::from_bytes(&payer.to_bytes()).expect("Payer error in client creation")),
        commitment_config,
    );
    let amm_program = client.program(lb_clmm::ID).unwrap();

    
    let lb_pair_state: LbPair = amm_program.account(lb_pair).await.unwrap();

    //Bin array bitmap extension
    let (bin_array_bitmap_extension, _bump) = derive_bin_array_bitmap_extension(lb_pair);
    let bin_array_bitmap_extension = if amm_program
        .rpc()
        .get_account(&bin_array_bitmap_extension)
        .is_err()
    {
        None
    } else {
        Some(bin_array_bitmap_extension)
    };

    //Get pda user token derived, not sure about that...
    let active_bin_array_idx = BinArray::bin_id_to_bin_array_index(lb_pair_state.active_id).unwrap();
    let (bin_array_0, _bump) = derive_bin_array_pda(lb_pair, active_bin_array_idx as i64);

    let (user_token_in, user_token_out, bin_array_1, bin_array_2) = if swap_for_y {
        (
            get_associated_token_address(&amm_program.payer(), &lb_pair_state.token_x_mint),
            get_associated_token_address(&amm_program.payer(), &lb_pair_state.token_y_mint),
            derive_bin_array_pda(lb_pair, (active_bin_array_idx - 1) as i64).0,
            derive_bin_array_pda(lb_pair, (active_bin_array_idx - 2) as i64).0,
        )
    } else {
        (
            get_associated_token_address(&amm_program.payer(), &lb_pair_state.token_y_mint),
            get_associated_token_address(&amm_program.payer(), &lb_pair_state.token_x_mint),
            derive_bin_array_pda(lb_pair, (active_bin_array_idx + 1) as i64).0,
            derive_bin_array_pda(lb_pair, (active_bin_array_idx + 2) as i64).0,
        )
    };

    //Get event authority
    let (event_authority, _bump) = Pubkey::find_program_address(&[b"__event_authority"], &lb_clmm::ID);
    
    let accounts = accounts::Swap {
        lb_pair,
        bin_array_bitmap_extension,
        reserve_x: lb_pair_state.reserve_x,
        reserve_y: lb_pair_state.reserve_y,
        token_x_mint: lb_pair_state.token_x_mint,
        token_y_mint: lb_pair_state.token_y_mint,
        token_x_program: anchor_spl::token::ID,
        token_y_program: anchor_spl::token::ID,
        user: amm_program.payer(),
        user_token_in,
        user_token_out,
        oracle: lb_pair_state.oracle,
        host_fee_in: Some(lb_clmm::ID),
        event_authority,
        program: lb_clmm::ID,
    };

    let ix = instruction::Swap {
        amount_in,
        min_amount_out: min_amount_out,
    };

    // I don't know what is this
    let remaining_accounts = vec![
        AccountMeta {
            is_signer: false,
            is_writable: true,
            pubkey: bin_array_0,
        },
        AccountMeta {
            is_signer: false,
            is_writable: true,
            pubkey: bin_array_1,
        },
        AccountMeta {
            is_signer: false,
            is_writable: true,
            pubkey: bin_array_2,
        },
    ];

    // Set a specific compute unit limit that the transaction is allowed to consume.
    // https://www.rareskills.io/post/solana-compute-unit-price
    let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);

    // let request_builder = amm_program.request();
    // let signature = request_builder
    //     .instruction(compute_budget_ix)
    //     .accounts(accounts)
    //     .accounts(remaining_accounts)
    //     .args(ix)
    //     .send_with_spinner_and_config(transaction_config)
    //     .await;

    return (compute_budget_ix, accounts, remaining_accounts, ix);
}

// pub async fn swap<C: Deref<Target = impl Signer> + Clone>(
//     params: SwapParameters,
//     program: &Program<C>,
//     transaction_config: RpcSendTransactionConfig,
// ) -> Result<()> {
//     let SwapParameters {
//         amount_in,
//         lb_pair,
//         swap_for_y,
//     } = params;

//     let lb_pair_state: LbPair = program.account(lb_pair).await?;

//     let active_bin_array_idx = BinArray::bin_id_to_bin_array_index(lb_pair_state.active_id)?;
//     let (bin_array_0, _bump) = derive_bin_array_pda(lb_pair, active_bin_array_idx as i64);

//     let (user_token_in, user_token_out, bin_array_1, bin_array_2) = if swap_for_y {
//         (
//             get_associated_token_address(&program.payer(), &lb_pair_state.token_x_mint),
//             get_associated_token_address(&program.payer(), &lb_pair_state.token_y_mint),
//             derive_bin_array_pda(lb_pair, (active_bin_array_idx - 1) as i64).0,
//             derive_bin_array_pda(lb_pair, (active_bin_array_idx - 2) as i64).0,
//         )
//     } else {
//         (
//             get_associated_token_address(&program.payer(), &lb_pair_state.token_y_mint),
//             get_associated_token_address(&program.payer(), &lb_pair_state.token_x_mint),
//             derive_bin_array_pda(lb_pair, (active_bin_array_idx + 1) as i64).0,
//             derive_bin_array_pda(lb_pair, (active_bin_array_idx + 2) as i64).0,
//         )
//     };

//     let (bin_array_bitmap_extension, _bump) = derive_bin_array_bitmap_extension(lb_pair);
//     let bin_array_bitmap_extension = if program
//         .rpc()
//         .get_account(&bin_array_bitmap_extension)
//         .is_err()
//     {
//         None
//     } else {
//         Some(bin_array_bitmap_extension)
//     };

//     let (event_authority, _bump) =
//         Pubkey::find_program_address(&[b"__event_authority"], &lb_clmm::ID);

// let accounts = accounts::Swap {
//     lb_pair,
//     bin_array_bitmap_extension,
//     reserve_x: lb_pair_state.reserve_x,
//     reserve_y: lb_pair_state.reserve_y,
//     token_x_mint: lb_pair_state.token_x_mint,
//     token_y_mint: lb_pair_state.token_y_mint,
//     token_x_program: anchor_spl::token::ID,
//     token_y_program: anchor_spl::token::ID,
//     user: program.payer(),
//     user_token_in,
//     user_token_out,
//     oracle: lb_pair_state.oracle,
//     host_fee_in: Some(lb_clmm::ID),
//     event_authority,
//     program: lb_clmm::ID,
// };

// let ix = instruction::Swap {
//     amount_in,
//     min_amount_out: min_amount_out,
// };

//     let remaining_accounts = vec![
//         AccountMeta {
//             is_signer: false,
//             is_writable: true,
//             pubkey: bin_array_0,
//         },
//         AccountMeta {
//             is_signer: false,
//             is_writable: true,
//             pubkey: bin_array_1,
//         },
//         AccountMeta {
//             is_signer: false,
//             is_writable: true,
//             pubkey: bin_array_2,
//         },
//     ];

//     let compute_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);

//     let request_builder = program.request();
//     let signature = request_builder
//         .instruction(compute_budget_ix)
//         .accounts(accounts)
//         .accounts(remaining_accounts)
//         .args(ix)
//         .send_with_spinner_and_config(transaction_config)
//         .await;

//     println!("Swap. Signature: {:#?}", signature);

//     signature?;

//     Ok(())
// }