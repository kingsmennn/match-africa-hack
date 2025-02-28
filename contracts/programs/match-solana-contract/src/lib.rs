pub mod constants;
pub mod events;
pub mod states;
pub mod errors;
use anchor_lang::prelude::*;
use solana_program::system_instruction;
use anchor_spl::
    token_interface::{
        TokenAccount, Mint,
        TokenInterface, TransferChecked, transfer_checked
    }
;
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};
use solana_program::pubkey::Pubkey;
declare_id!("Gde2V9pUafhQSDMYNMhtGTjTDDWBvD9DZH251Hq26wgL");
use crate::{constants::*, events::*, states::*, errors::*};
use solana_program::pubkey;
use std::mem::size_of;
const PYTH_USDC_FEED: Pubkey = pubkey!("7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE");
pub const MAXIMUM_AGE: u64 = 60;
const SOL_USD_PRICE_FEED:&str = "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d";
#[program]
pub mod marketplace {
    use super::*;

    pub fn initialize_counters(ctx: Context<InitializeCounters>) -> Result<()> {
        let user_counter = &mut ctx.accounts.user_counter;
        let store_counter = &mut ctx.accounts.store_counter;
        let request_counter = &mut ctx.accounts.request_counter;
        let offer_counter = &mut ctx.accounts.offer_counter;
    
        user_counter.current = 1;
        store_counter.current = 1;
        request_counter.current = 1;
        offer_counter.current = 1;
    
        msg!("Counters initialized: Users, Stores, Requests, Offers");
        
        Ok(())
    }
    pub fn initialize_counters_pay(ctx: Context<InitializePaymentIncrement>) -> Result<()> {
        let request_payment_counter = &mut ctx.accounts.request_payment_counter;

        request_payment_counter.current = 1;
    
        msg!("Counters initialized: PaymentIncrement");
        
        Ok(())
    }

    pub fn create_user(
        ctx: Context<CreateUser>,
        username: String,
        phone: String,
        latitude: i128,
        longitude: i128,
        account_type: AccountType,
    ) -> Result<()> {
        let user = &mut ctx.accounts.user;
        let user_counter = &mut ctx.accounts.user_counter;

        if user.id != 0 {
            return err!(MarketplaceError::UserAlreadyExists);
        }

        if account_type != AccountType::Buyer && account_type != AccountType::Seller {
            return err!(MarketplaceError::InvalidAccountType);
        }
     

        user.id = user_counter.current;

        user_counter.current = user_counter.current.checked_add(1).unwrap();

        user.username = username;
        user.phone = phone;
        user.location = Location {
            latitude,
            longitude,
        };
        user.account_type = account_type;
        user.created_at = Clock::get().unwrap().unix_timestamp;
        user.updated_at = Clock::get().unwrap().unix_timestamp;
        user.authority = ctx.accounts.authority.key();
        user.location_enabled = true;

        msg!("UserCreated: {:?}", user);

        Ok(())
    }

    pub fn update_user(
        ctx: Context<UpdateUser>,
        username: String,
        phone: String,
        latitude: i128,
        longitude: i128,
        account_type: AccountType,
    ) -> Result<()> {
        let user = &mut ctx.accounts.user;

        if user.id == 0 {
            return err!(MarketplaceError::InvalidUser);
        }

        user.username = username;
        user.phone = phone;
        user.location = Location {
            latitude,
            longitude,
        };
        user.updated_at = Clock::get().unwrap().unix_timestamp;
        user.account_type = account_type;
        user.authority = ctx.accounts.authority.key();

        msg!("UserUpdated: {:?}", user);

        Ok(())
    }


    pub fn create_store(
        ctx: Context<CreateStore>,
        name: String,
        description: String,
        phone: String,
        latitude: i128,
        longitude: i128
    ) -> Result<()> {
        let user = &mut ctx.accounts.user;
        let store_counter = &mut ctx.accounts.store_counter;

        if user.account_type != AccountType::Seller {
            return err!(MarketplaceError::OnlySellersAllowed);
        }

        let store = &mut ctx.accounts.store;

        store.id = store_counter.current;
        store.name = name;
        store.description = description;
        store.phone = phone;
        store.location = Location {
            latitude,
            longitude,
        };
        store.authority = ctx.accounts.authority.key();

        store_counter.current = store_counter.current.checked_add(1).unwrap();

        emit!(StoreCreated {
            seller_address: *ctx.accounts.user.to_account_info().key,
            store_id: store.id,
            store_name: store.name.clone(),
            latitude: store.location.latitude,
            longitude: store.location.longitude,
        });

        Ok(())
    }

    pub fn create_request(
        ctx: Context<CreateRequest>,
        name: String,
        description: String,
        images: Vec<String>,
        latitude: i128,
        longitude: i128,
    ) -> Result<()> {
        let user = &mut ctx.accounts.user;
        let request_counter = &mut ctx.accounts.request_counter;

        if user.account_type != AccountType::Buyer {
            return err!(MarketplaceError::OnlyBuyersAllowed);
        }

        let request = &mut ctx.accounts.request;

        request.id = request_counter.current;
        request.name = name;
        request.buyer_id = user.id;
        request.sellers_price_quote = 0;
        request.seller_ids = Vec::new();
        request.offer_ids = Vec::new();
        request.locked_seller_id = 0;
        request.description = description;
        request.images = images;
        request.created_at = Clock::get().unwrap().unix_timestamp as u64;
        request.lifecycle = RequestLifecycle::Pending;
        request.location = Location {
            latitude,
            longitude,
        };
        request.paid = false;
        request.updated_at = Clock::get().unwrap().unix_timestamp as u64;
        request.authority = ctx.accounts.authority.key();

        request_counter.current = request_counter.current.checked_add(1).unwrap();

        emit!(RequestCreated {
            request_id: request.id,
            buyer_address: *ctx.accounts.user.to_account_info().key,
            request_name: request.name.clone(),
            latitude: request.location.latitude,
            longitude: request.location.longitude,
            images: request.images.clone(),
            lifecycle: request.lifecycle.clone() as u8,
            description: request.description.clone(),
            buyer_id: request.buyer_id,
            seller_ids: request.seller_ids.clone(),
            sellers_price_quote: request.sellers_price_quote,
            locked_seller_id: request.locked_seller_id,
            created_at: request.created_at,
            updated_at: request.updated_at,
        });

        Ok(())
    }

    pub fn delete_request(ctx: Context<RemoveRequest>) -> Result<()> {
        let request = &mut ctx.accounts.request;
        let authority = &ctx.accounts.authority;
    
        if request.authority != authority.key() {
            return err!(MarketplaceError::InvalidUser);
        }
    
        if request.lifecycle != RequestLifecycle::Pending {
            return err!(MarketplaceError::RequestLocked);
        }
    
        Ok(())
    }

    pub fn mark_request_as_completed(ctx: Context<MarkAsCompleteRequest>) -> Result<()> {
        let request = &mut ctx.accounts.request;
        let mint = &ctx.accounts.mint;
        let token_program = &ctx.accounts.token_program;
        let to_ata = &ctx.accounts.to_ata;
        let authority = &ctx.accounts.authority;
        let request_payment_info = &mut ctx.accounts.request_payment_info;

        // let (pda, bump) = Pubkey::find_program_address(
        //     &[REQUEST_PAYMENT_TAG],
        //     ctx.program_id
        // );

        let bump_seed = ctx.bumps.request_payment_counter;
        let signer_seeds : &[&[&[u8]]] = &[&[REQUEST_PAYMENT_TAG, &[bump_seed]]];


        let accounts = TransferChecked {
            from: ctx.accounts.token_ata.to_account_info(),
            to: to_ata.to_account_info(),
            authority: ctx.accounts.request_payment_counter.to_account_info(),
            mint: mint.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            token_program.to_account_info(),
            accounts,
            signer_seeds
        );

        
    
        if request.authority != authority.key() {
            return err!(MarketplaceError::InvalidUser);
        }
    
        if request.lifecycle != RequestLifecycle::Paid {
            return err!(MarketplaceError::RequestNotPaid);
        }

        if request.updated_at + TIME_TO_LOCK  > Clock::get().unwrap().unix_timestamp as u64 {
            return err!(MarketplaceError::RequestNotLocked);
        }
    
        request.lifecycle = RequestLifecycle::Completed;
        request.updated_at = Clock::get().unwrap().unix_timestamp as u64;

        let pyusd_amount = request_payment_info.amount;

        transfer_checked(ctx, pyusd_amount, mint.decimals)?;
    
        Ok(())
    }

    pub fn pay_for_request_token(ctx: Context<PayForRequestToken>,coin: CoinPayment) -> Result<()> {
        let request = &mut ctx.accounts.request;
        let offer = &mut ctx.accounts.offer;
        let authority = &ctx.accounts.authority;
        let mint = &ctx.accounts.mint;
        let from_ata = &ctx.accounts.from_ata;
        let to_ata = &ctx.accounts.to_ata;
        let token_program = &ctx.accounts.token_program;
        let request_payment_counter = &mut ctx.accounts.request_payment_counter;
        let request_payment_info = &mut ctx.accounts.request_payment_info;
    
        if request.authority != authority.key() {
            return err!(MarketplaceError::InvalidUser);
        }
    
        if request.lifecycle != RequestLifecycle::AcceptedByBuyer {
            return err!(MarketplaceError::RequestNotAccepted);
        }

        if request.updated_at + TIME_TO_LOCK  > Clock::get().unwrap().unix_timestamp as u64 {
            return err!(MarketplaceError::RequestNotLocked);
        }

        if !offer.is_accepted {
            return err!(MarketplaceError::RequestNotAccepted);
        }


        if request.locked_seller_id != offer.seller_id {
            return err!(MarketplaceError::InvalidSeller);
        }

        if request.paid {
            return err!(MarketplaceError::RequestAlreadyPaid);
        }

        request.paid = true;
        request.lifecycle = RequestLifecycle::Paid;
        request_payment_info.authority = authority.key();
        request_payment_info.request_id = request.id;
        request_payment_info.buyer_id = request.buyer_id;
        request_payment_info.price = offer.price;
        request_payment_info.seller_id = offer.seller_id;
        request_payment_info.created_at = Clock::get().unwrap().unix_timestamp as u64;
        request_payment_info.updated_at = Clock::get().unwrap().unix_timestamp as u64;
        request_payment_info.token = coin.clone();
        request_payment_info.id = request_payment_counter.current;
        request_payment_info.seller_authority = offer.authority;
        request_payment_counter.current = request_payment_counter.current.checked_add(1).unwrap();
        


        match coin {
            CoinPayment::Pyusdt => {
                let price_update = &mut ctx.accounts.price_update;
                let current_price = price_update.get_price_no_older_than(
                    &Clock::get()?,
                    MAXIMUM_AGE,
                    &get_feed_id_from_hex(SOL_USD_PRICE_FEED)?,
                )?;
        
                let sol_price_in_usd = current_price.price as u64;
                let sol_price_for_offer = offer.price;
        
                let sol_amount_in_usd = sol_price_for_offer * sol_price_in_usd;
                
                let pyusd_amount = sol_amount_in_usd / 100000000000;

                request_payment_info.amount = pyusd_amount;

                let accounts = TransferChecked {
                    from: from_ata.to_account_info(),
                    to: to_ata.to_account_info(),
                    authority: authority.to_account_info(),
                    mint: mint.to_account_info(),
                };
                
                let ctx = CpiContext::new(
                    token_program.to_account_info(),
                    accounts
                );

                transfer_checked(ctx, pyusd_amount, mint.decimals)?;
            },
            _ => {
                return err!(MarketplaceError::InvalidCoinPayment);
            },
            
        }
        
        Ok(())
    }


    pub fn pay_for_request(ctx: Context<PayForRequest>,coin: CoinPayment) -> Result<()> {
        let request = &mut ctx.accounts.request;
        let offer = &mut ctx.accounts.offer;
        let to = &mut ctx.accounts.to;
        let authority = &ctx.accounts.authority;
        let request_payment_counter = &mut ctx.accounts.request_payment_counter;
        let request_payment_info = &mut ctx.accounts.request_payment_info;
    
        if request.authority != authority.key() {
            return err!(MarketplaceError::InvalidUser);
        }
    
        if request.lifecycle != RequestLifecycle::AcceptedByBuyer {
            return err!(MarketplaceError::RequestNotAccepted);
        }

        if request.updated_at + TIME_TO_LOCK  > Clock::get().unwrap().unix_timestamp as u64 {
            return err!(MarketplaceError::RequestNotLocked);
        }

        if !offer.is_accepted {
            return err!(MarketplaceError::RequestNotAccepted);
        }


        if request.locked_seller_id != offer.seller_id {
            return err!(MarketplaceError::InvalidSeller);
        }

        if request.paid {
            return err!(MarketplaceError::RequestAlreadyPaid);
        }

        request.paid = true;
        request.lifecycle = RequestLifecycle::Paid;
        request_payment_info.authority = authority.key();
        request_payment_info.request_id = request.id;
        request_payment_info.buyer_id = request.buyer_id;
        request_payment_info.price = offer.price;
        request_payment_info.seller_authority = offer.authority;
        request_payment_info.seller_id = offer.seller_id;
        request_payment_info.created_at = Clock::get().unwrap().unix_timestamp as u64;
        request_payment_info.updated_at = Clock::get().unwrap().unix_timestamp as u64;
        request_payment_info.token = coin.clone();
        request_payment_info.id = request_payment_counter.current;
        request_payment_counter.current = request_payment_counter.current.checked_add(1).unwrap();

        match coin {
            CoinPayment::Solana => {
                let transfer_instruction = system_instruction::transfer(authority.key, to.key, offer.price);

                request_payment_info.amount = offer.price;

                anchor_lang::solana_program::program::invoke_signed(
                    &transfer_instruction,
                    &[
                        authority.to_account_info(),
                        to.clone(),
                        ctx.accounts.system_program.to_account_info(),
                    ],
                    &[],
                )?;
            }
            _ => {
                return err!(MarketplaceError::InvalidCoinPayment);
            },
        }
        Ok(())
    }

    pub fn toggle_location(ctx: Context<ToggleLocation>, enabled: bool) -> Result<()> {
        let  user  = &mut ctx.accounts.user;

        user.location_enabled = enabled;

        emit!(LocationEnabled {
            location_enabled: enabled,
            user_id: user.id,
        });
        
        Ok(())
    }

    pub fn get_location_preference(ctx: Context<GetLocationPreference>) -> Result<bool> {
        let user = &ctx.accounts.user;
        
        Ok(user.location_enabled)
    }


    pub fn create_offer(
        ctx: Context<CreateOffer>,
        price: u64,
        images: Vec<String>,
        store_name: String,
    ) -> Result<()> {
        let user = &mut ctx.accounts.user;
        let offer_counter = &mut ctx.accounts.offer_counter;

        if user.account_type != AccountType::Seller {
            return err!(MarketplaceError::OnlySellersAllowed);
        }

        let request = &mut ctx.accounts.request;

        if Clock::get().unwrap().unix_timestamp as u64 > request.updated_at + TIME_TO_LOCK
            && request.lifecycle == RequestLifecycle::AcceptedByBuyer
        {
            return err!(MarketplaceError::RequestLocked);
        }

        let offer = &mut ctx.accounts.offer;

        offer.id = offer_counter.current;
        offer.price = price;
        offer.images = images;
        offer.request_id = request.id;
        offer.store_name = store_name;
        offer.seller_id = user.id;
        offer.is_accepted = false;
        offer.created_at = Clock::get().unwrap().unix_timestamp as u64;
        offer.updated_at = Clock::get().unwrap().unix_timestamp as u64;
        offer.authority = ctx.accounts.authority.key();

        if request.lifecycle == RequestLifecycle::Pending {
            request.lifecycle = RequestLifecycle::AcceptedBySeller;
        }

        request.seller_ids.push(offer.seller_id);

        emit!(OfferCreated {
            offer_id: offer.id,
            seller_address: *ctx.accounts.user.to_account_info().key,
            store_name: offer.store_name.clone(),
            price: offer.price,
            request_id: offer.request_id,
            images: offer.images.clone(),
            seller_id: offer.seller_id,
            seller_ids: request.seller_ids.clone(),
        });

        offer_counter.current = offer_counter.current.checked_add(1).unwrap();

        Ok(())
    }

    pub fn accept_offer(ctx: Context<AcceptOffer>) -> Result<()> {
        let user = &mut ctx.accounts.user;
        let offer = &mut ctx.accounts.offer;
        let request = &mut ctx.accounts.request;

        if user.account_type != AccountType::Buyer {
            return err!(MarketplaceError::OnlyBuyersAllowed);
        }

        if request.buyer_id != user.id {
            return err!(MarketplaceError::UnauthorizedBuyer);
        }

        if offer.is_accepted {
            return err!(MarketplaceError::OfferAlreadyAccepted);
        }

        if Clock::get().unwrap().unix_timestamp as u64 > request.updated_at + TIME_TO_LOCK
            && request.lifecycle == RequestLifecycle::AcceptedByBuyer
        {
            return err!(MarketplaceError::RequestLocked);
        }


        if request.seller_ids.len() != ctx.remaining_accounts.len() {
            return err!(MarketplaceError::IncorrectNumberOfSellers);
        }

        for account_info in ctx.remaining_accounts.iter() {
            let mut data = account_info.try_borrow_mut_data()?;

            let mut previous_offer = Offer::try_deserialize(&mut data.as_ref()).expect("Error Deserializing Data");
            
            if previous_offer.is_accepted  && previous_offer.request_id == request.id {
                previous_offer.is_accepted = false;
                previous_offer.try_serialize(&mut data.as_mut())?;
                emit!(OfferAccepted {
                    offer_id: previous_offer.id,
                    buyer_address: *ctx.accounts.user.to_account_info().key,
                    is_accepted: false,
                });
            }
        }

        offer.is_accepted = true;
        offer.updated_at = Clock::get().unwrap().unix_timestamp as u64;
        request.offer_ids.push(offer.id);
        request.locked_seller_id = offer.seller_id;
        request.sellers_price_quote = offer.price;
        request.accepted_offer_id = offer.id;
        request.lifecycle = RequestLifecycle::AcceptedByBuyer;
        request.updated_at = Clock::get().unwrap().unix_timestamp as u64;

        emit!(RequestAccepted {
            request_id: request.id,
            offer_id: offer.id,
            seller_id: offer.seller_id,
            updated_at: request.updated_at,
            sellers_price_quote: request.sellers_price_quote,
        });

        emit!(OfferAccepted {
            offer_id: offer.id,
            buyer_address: *ctx.accounts.user.to_account_info().key,
            is_accepted: true,
        });

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction()]
pub struct CreateUser<'info> {
    #[account(
        init,
        seeds = [USER_TAG,authority.key.as_ref()],
        bump,
        payer = authority,
        space = 8 + size_of::<User>() + 1024)]
    pub user: Box<Account<'info, User>>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [USER_COUNTER],
        bump,
    )]
    pub user_counter: Box<Account<'info, Counter>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction()]
pub struct UpdateUser<'info> {
    #[account(
        mut,
        seeds = [USER_TAG,authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub user: Box<Account<'info, User>>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction()]
pub struct CreateStore<'info> {
    #[account(
        mut,
        seeds = [USER_TAG,authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub user: Box<Account<'info, User>>,
    #[account(init, payer = authority ,space = 8 + size_of::<Store>() + 1024,        
    seeds = [STORE_TAG, authority.key().as_ref(),&store_counter.current.to_le_bytes()],
    bump,)]
    pub store: Box<Account<'info, Store>>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [STORE_COUNTER],
        bump,
    )]
    pub store_counter: Box<Account<'info, Counter>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction()]
pub struct CreateRequest<'info> {
    #[account(
        mut,
        seeds = [USER_TAG,authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub user: Box<Account<'info, User>>,
    #[account(init, payer = authority ,space = 8 + size_of::<Request>() + 1024,        
    seeds = [REQUEST_TAG, authority.key().as_ref(),&request_counter.current.to_le_bytes()],
    bump,)]
    pub request: Box<Account<'info, Request>>,
    #[account(
        mut,
        seeds = [REQUEST_COUNTER],
        bump,
    )]
    pub request_counter: Box<Account<'info, Counter>>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveRequest<'info> {
    #[account(
        mut,
        has_one = authority,
        seeds = [REQUEST_TAG, authority.key().as_ref(), &request.id.to_le_bytes()],
        bump,
        close = authority
    )]
    pub request: Box<Account<'info, Request>>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ToggleLocation<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [USER_TAG,authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub user: Box<Account<'info, User>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetLocationPreference<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [USER_TAG,authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub user: Box<Account<'info, User>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MarkAsCompleteRequest<'info> {
    #[account(
        mut,
        has_one = authority,
        seeds = [REQUEST_TAG, authority.key().as_ref(), &request.id.to_le_bytes()],
        bump,
    )]
    pub request: Box<Account<'info, Request>>,

    #[account(mut, 
    seeds = [REQUEST_PAYMENT_TAG, authority.key().as_ref(),&request_payment_counter.current.to_le_bytes()],
    bump,)]
    pub request_payment_info: Box<Account<'info, RequestPaymentTransaction>>,

    #[account(
        mut,
        seeds = [REQUEST_PAYMENT_COUNTER],
        bump,
    )]
    pub request_payment_counter: Box<Account<'info, Counter>>,

    pub mint: InterfaceAccount<'info, Mint>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub to_ata: InterfaceAccount<'info, TokenAccount>,

    // #[account(
    //     init_if_needed,
    //     payer = authority,
    //     associated_token::mint = mint,
    //     associated_token::authority = request_payment_counter,
    // )]
    #[account(mut)]
    pub token_ata: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    
    pub token_program: Interface<'info, TokenInterface>,
}
#[derive(Accounts)]
pub struct PayForRequest<'info> {
    #[account(
        mut,
        has_one = authority,
        seeds = [REQUEST_TAG, authority.key().as_ref(), &request.id.to_le_bytes()],
        bump,
    )]
    pub request: Box<Account<'info, Request>>,

    #[account(
        mut,
    )]
    pub offer: Box<Account<'info, Offer>>,

    #[account(init, payer = authority ,space = 8 + size_of::<RequestPaymentTransaction>() + 1024,   
    seeds = [REQUEST_PAYMENT_TAG, authority.key().as_ref(),&request_payment_counter.current.to_le_bytes()],
    bump,)]
    pub request_payment_info: Box<Account<'info, RequestPaymentTransaction>>,

    #[account(
        mut,
        seeds = [REQUEST_PAYMENT_COUNTER],
        bump,
    )]
    pub request_payment_counter: Box<Account<'info, Counter>>,
    
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: This is the account to which the payment is made
    #[account(mut, address = ID)]
    pub to: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct PayForRequestToken<'info> {
    #[account(
        mut,
        has_one = authority,
        seeds = [REQUEST_TAG, authority.key().as_ref(), &request.id.to_le_bytes()],
        bump,
    )]
    pub request: Box<Account<'info, Request>>,

    #[account(init, payer = authority ,space = 8 + size_of::<RequestPaymentTransaction>() + 1024,   
    seeds = [REQUEST_PAYMENT_TAG, authority.key().as_ref(),&request_payment_counter.current.to_le_bytes()],
    bump,)]
    pub request_payment_info: Box<Account<'info, RequestPaymentTransaction>>,

    #[account(
        mut,
        seeds = [REQUEST_PAYMENT_COUNTER],
        bump,
    )]
    pub request_payment_counter: Box<Account<'info, Counter>>,

    #[account(
        mut,
    )]
    pub offer: Box<Account<'info, Offer>>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub from_ata: InterfaceAccount<'info, TokenAccount>,

    // #[account(
    //     init_if_needed,
    //     payer = authority,
    //     associated_token::mint = mint,
    //     associated_token::authority = request_payment_counter,
    // )]
    #[account(mut)]
    pub to_ata: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, anchor_spl::associated_token::AssociatedToken>,
    
    pub token_program: Interface<'info, TokenInterface>,

    /// CHECK: this is the price feed
    #[account(address = PYTH_USDC_FEED)]
    pub price_update: Account<'info, PriceUpdateV2>,


    pub mint: InterfaceAccount<'info, Mint>,
}

#[derive(Accounts)]
#[instruction()]
pub struct CreateOffer<'info> {
    #[account(
        mut,
        seeds = [USER_TAG,authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub user: Box<Account<'info, User>>,
    #[account(mut)]
    pub request: Box<Account<'info, Request>>,
    #[account(init, payer = authority ,space = 8 + size_of::<Offer>() + 1024,     
    seeds = [OFFER_TAG, authority.key().as_ref(),&offer_counter.current.to_le_bytes()],
    bump,)]
    pub offer: Box<Account<'info, Offer>>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [OFFER_COUNTER],
        bump,
    )]
    pub offer_counter: Box<Account<'info, Counter>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction()]
pub struct AcceptOffer<'info> {
    #[account(
        mut,
        seeds = [USER_TAG,authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub user: Box<Account<'info, User>>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub offer: Box<Account<'info, Offer>>,
    #[account(mut)]
    pub request: Box<Account<'info, Request>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializePaymentIncrement<'info> {
    #[account(
        init,
        seeds = [REQUEST_PAYMENT_COUNTER],
        bump,
        payer = authority,
        space = 8 + size_of::<Counter>()
    )]
    pub request_payment_counter: Box<Account<'info, Counter>>,

    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
pub struct InitializeCounters<'info> {
    #[account(
        init,
        seeds = [USER_COUNTER],
        bump,
        payer = authority,
        space = 8 + size_of::<Counter>()
    )]
    pub user_counter: Box<Account<'info, Counter>>,

    #[account(
        init,
        seeds = [STORE_COUNTER],
        bump,
        payer = authority,
        space = 8 + size_of::<Counter>()
    )]
    pub store_counter: Box<Account<'info, Counter>>,

    #[account(
        init,
        seeds = [REQUEST_COUNTER],
        bump,
        payer = authority,
        space = 8 + size_of::<Counter>()
    )]
    pub request_counter: Box<Account<'info, Counter>>,

    #[account(
        init,
        seeds = [OFFER_COUNTER],
        bump,
        payer = authority,
        space = 8 + size_of::<Counter>()
    )]
    pub offer_counter: Box<Account<'info, Counter>>,

    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}


