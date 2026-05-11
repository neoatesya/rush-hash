use std::env;
use std::str::FromStr;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use std::fs::OpenOptions;
use std::io::Write;

use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::transaction::eip1559::Eip1559TransactionRequest;
use ocl::{ProQue, Buffer, flags};

const CONTRACT_ADDRESS: &str = "0xAC7b5d06fa1e77D08aea40d46cB7C5923A87A0cc";

abigen!(
    Hash256Contract,
    r#"[
        function getChallenge(address miner) view returns (bytes32)
        function miningState() view returns (uint256 era,uint256 reward,uint256 difficulty,uint256 minted,uint256 remaining,uint256 epoch,uint256 epochBlocksLeft_)
        function mine(uint256 nonce)
    ]"#,
);

const KERNEL_SRC: &str = r#"
#pragma OPENCL EXTENSION cl_khr_int64_base_atomics : enable
typedef struct{uint found;uint nonce_lo;uint nonce_hi;uint hash[8];} Result;
__constant ulong RC[24]={0x0000000000000001UL,0x0000000000008082UL,0x800000000000808aUL,0x8000000080008000UL,0x000000000000808bUL,0x0000000080000001UL,0x8000000080008081UL,0x8000000000008009UL,0x000000000000008aUL,0x0000000000000088UL,0x0000000080008009UL,0x000000008000000aUL,0x000000008000808bUL,0x800000000000008bUL,0x8000000000008089UL,0x8000000000008003UL,0x8000000000008002UL,0x8000000000000080UL,0x000000000000800aUL,0x800000008000000aUL,0x8000000080008081UL,0x8000000000008080UL,0x0000000080000001UL,0x8000000080008008UL};
__constant int R[24]={1,3,6,10,15,21,28,36,45,55,2,14,27,41,56,8,25,43,62,18,39,61,20,44};
__constant int P[24]={10,7,11,17,18,3,5,16,8,21,24,4,15,23,19,13,12,2,20,14,22,9,6,1};
uint bswap32(uint v){return ((v&255U)<<24)|((v&65280U)<<8)|((v&16711680U)>>8)|((v&4278190080U)>>24);}
ulong rotl64(ulong x,int s){return rotate(x,(ulong)s);}
void keccakf(ulong st[25]){int i,j,r;ulong t,bc[5];for(r=0;r<24;r++){for(i=0;i<5;i++)bc[i]=st[i]^st[i+5]^st[i+10]^st[i+15]^st[i+20];for(i=0;i<5;i++){t=bc[(i+4)%5]^rotl64(bc[(i+1)%5],1);for(j=0;j<25;j+=5)st[j+i]^=t;}t=st[1];for(i=0;i<24;i++){j=P[i];bc[0]=st[j];st[j]=rotl64(t,R[i]);t=bc[0];}for(j=0;j<25;j+=5){for(i=0;i<5;i++)bc[i]=st[j+i];for(i=0;i<5;i++)st[j+i]^=(~bc[(i+1)%5])&bc[(i+2)%5];}st[0]^=RC[r];}}
int below(uint h[8],__global const uint *d){for(int i=0;i<8;i++){if(h[i]<d[i])return 1;if(h[i]>d[i])return 0;}return 0;}
__kernel void mine(__global const uint *challenge,__global const uint *difficulty,ulong base,__global Result *out){size_t gid=get_global_id(0);ulong nonce=base+(ulong)gid;ulong st[25];for(int i=0;i<25;i++)st[i]=0UL;st[0]=((ulong)challenge[1]<<32)|challenge[0];st[1]=((ulong)challenge[3]<<32)|challenge[2];st[2]=((ulong)challenge[5]<<32)|challenge[4];st[3]=((ulong)challenge[7]<<32)|challenge[6];uint lo=(uint)(nonce&0xffffffffUL);uint hi=(uint)(nonce>>32);st[7]=((ulong)bswap32(lo)<<32)|bswap32(hi);st[8]=1UL;st[16]=0x8000000000000000UL;keccakf(st);uint h[8];h[0]=bswap32((uint)(st[0]&0xffffffffUL));h[1]=bswap32((uint)(st[0]>>32));h[2]=bswap32((uint)(st[1]&0xffffffffUL));h[3]=bswap32((uint)(st[1]>>32));h[4]=bswap32((uint)(st[2]&0xffffffffUL));h[5]=bswap32((uint)(st[2]>>32));h[6]=bswap32((uint)(st[3]&0xffffffffUL));h[7]=bswap32((uint)(st[3]>>32));if(below(h,difficulty)){if(atomic_cmpxchg((volatile __global unsigned int *)&out->found,0U,1U)==0U){out->nonce_lo=lo;out->nonce_hi=hi;for(int i=0;i<8;i++)out->hash[i]=h[i];}}}
"#;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct ResultBuffer {
    found: u32,
    nonce_lo: u32,
    nonce_hi: u32,
    hash: [u32; 8],
}
unsafe impl ocl::OclPrm for ResultBuffer {}

fn log_message(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let formatted = format!("[{}] {}\n", timestamp, msg);
    
    print!("{}", formatted);
    
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("miner.log") {
        let _ = file.write_all(formatted.as_bytes());
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    log_message("Starting Hash256 GPU Miner...");

    let rpc_urls_str = env::var("RPC_URL").expect("RPC_URL must be set in .env");
    let rpc_urls: Vec<&str> = rpc_urls_str.split(',').map(|s| s.trim()).collect();
    let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    
    let priority_fee_gwei = env::var("PRIORITY_FEE_GWEI")
        .unwrap_or_else(|_| "2".to_string())
        .parse::<u64>()
        .unwrap_or(2);
    let max_priority_fee_per_gas = U256::from(priority_fee_gwei) * U256::from(1_000_000_000u64);

    let mut clients = Vec::new();
    let mut primary_contract = None;
    let mut primary_provider = None;
    let mut primary_wallet_address = Address::zero();

    log_message("Connecting to RPCs...");
    for url in &rpc_urls {
        if let Ok(provider) = Provider::<Http>::try_from(*url) {
            let chain_id = provider.get_chainid().await.unwrap_or_else(|_| U256::from(1)).as_u64();
            let wallet = LocalWallet::from_str(&private_key)?.with_chain_id(chain_id);
            primary_wallet_address = wallet.address();
            let client = Arc::new(SignerMiddleware::new(provider.clone(), wallet));
            clients.push(client.clone());
            
            if primary_contract.is_none() {
                let contract_address = CONTRACT_ADDRESS.parse::<Address>()?;
                primary_contract = Some(Hash256Contract::new(contract_address, client));
                primary_provider = Some(provider);
            }
            log_message(&format!("Connected: {}", url));
        } else {
            log_message(&format!("Failed to connect to: {}", url));
        }
    }

    if clients.is_empty() {
        log_message("❌ Failed to connect to any RPC URL!");
        std::process::exit(1);
    }
    
    let contract = primary_contract.unwrap();
    let provider = primary_provider.unwrap();

    log_message(&format!("Wallet: {:?}", primary_wallet_address));
    log_message(&format!("Contract: {}", CONTRACT_ADDRESS));

    let batch_size: usize = 16_777_216;
    let pro_que = match ProQue::builder()
        .src(KERNEL_SRC)
        .dims(batch_size)
        .build() {
        Ok(pq) => pq,
        Err(e) => {
            log_message(&format!("Failed to initialize OpenCL GPU: {:?}", e));
            log_message("Make sure you have OpenCL drivers installed (`sudo apt install ocl-icd-opencl-dev` or GPU drivers).");
            std::process::exit(1);
        }
    };
    
    log_message(&format!("OpenCL GPU Initialized: {:?}", pro_que.device().name().unwrap_or_default()));

    let challenge_buf = Buffer::<u32>::builder()
        .queue(pro_que.queue().clone())
        .flags(flags::MEM_READ_ONLY)
        .len(8)
        .build()?;
        
    let difficulty_buf = Buffer::<u32>::builder()
        .queue(pro_que.queue().clone())
        .flags(flags::MEM_READ_ONLY)
        .len(8)
        .build()?;
        
    let result_buf = Buffer::<ResultBuffer>::builder()
        .queue(pro_que.queue().clone())
        .flags(flags::MEM_READ_WRITE)
        .len(1)
        .build()?;

    let kernel = pro_que.kernel_builder("mine")
        .arg(&challenge_buf)
        .arg(&difficulty_buf)
        .arg(0u64)
        .arg(&result_buf)
        .build()?;

    loop {
        let state = match contract.mining_state().call().await {
            Ok(s) => s,
            Err(e) => {
                log_message(&format!("Error getting state: {:?}. Retrying...", e));
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        let difficulty = state.2; 
        
        let challenge = match contract.get_challenge(primary_wallet_address).call().await {
            Ok(c) => c,
            Err(e) => {
                log_message(&format!("Error getting challenge: {:?}. Retrying...", e));
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        
        let epoch = state.5;
        let chal_hex = hex::encode(challenge);
        let chal_short = &chal_hex[..8];

        log_message(&format!("Era: {} | Reward: {} HASH | Epoch: {}", state.0, ethers::utils::format_units(state.1, 18).unwrap_or_default(), epoch));
        log_message(&format!("Challenge: 0x{} | Difficulty: {}", chal_hex, difficulty));

        let mut diff_bytes = [0u8; 32];
        difficulty.to_big_endian(&mut diff_bytes);
        
        let mut chal_u32 = [0u32; 8];
        let mut diff_u32 = [0u32; 8];
        for i in 0..8 {
            chal_u32[i] = u32::from_le_bytes(challenge[i*4..(i+1)*4].try_into().unwrap());
            diff_u32[i] = u32::from_be_bytes(diff_bytes[i*4..(i+1)*4].try_into().unwrap());
        }
        
        challenge_buf.write(&chal_u32[..]).enq()?;
        difficulty_buf.write(&diff_u32[..]).enq()?;

        let found = Arc::new(AtomicBool::new(false));
        let challenge_changed = Arc::new(AtomicBool::new(false));

        let check_contract = contract.clone();
        let miner_address = primary_wallet_address;
        let challenge_checker = {
            let cc = challenge_changed.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    if cc.load(Ordering::Relaxed) {
                        break;
                    }
                    if let Ok(new_challenge) = check_contract.get_challenge(miner_address).call().await {
                        if new_challenge != challenge {
                            cc.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                }
            })
        };

        let mut base_nonce: u64 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64;
        let mut total_hashes: u64 = 0;
        let start_time = Instant::now();
        let mut last_print = Instant::now();

        let mut valid_nonce = U256::zero();
        
        while !challenge_changed.load(Ordering::Relaxed) {
            let clear_res = vec![ResultBuffer::default()];
            result_buf.write(&clear_res).enq()?;
            
            kernel.set_arg(2, base_nonce)?;
            unsafe {
                kernel.enq()?;
            }
            pro_que.queue().finish()?;

            let mut result_vec = vec![ResultBuffer::default()];
            result_buf.read(&mut result_vec).enq()?;

            if result_vec[0].found == 1 {
                let n = ((result_vec[0].nonce_hi as u64) << 32) | (result_vec[0].nonce_lo as u64);
                valid_nonce = U256::from(n);
                found.store(true, Ordering::Relaxed);
                challenge_changed.store(true, Ordering::Relaxed);
                break;
            }

            base_nonce = base_nonce.wrapping_add(batch_size as u64);
            total_hashes += batch_size as u64;
            
            if last_print.elapsed().as_secs_f64() > 2.0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                print!("\x1B[2K\r[Epoch {} | Chal: 0x{}..] {:.2} MH/s ({} hashes total)", 
                    epoch, chal_short, (total_hashes as f64 / elapsed) / 1_000_000.0, total_hashes);
                let _ = std::io::stdout().flush();
                last_print = Instant::now();
            }
        }
        println!();

        challenge_changed.store(true, Ordering::Relaxed);
        let _ = challenge_checker.await;

        if found.load(Ordering::Relaxed) {
            log_message(&format!("✅ FOUND nonce: {}", valid_nonce));
            
            let mut tx = contract.mine(valid_nonce).tx.clone();
            tx.set_gas(500_000);
            
            if let Ok((max_fee_per_gas, _)) = provider.estimate_eip1559_fees(None).await {
                if let TypedTransaction::Eip1559(ref mut inner) = tx {
                    inner.max_priority_fee_per_gas = Some(max_priority_fee_per_gas);
                    inner.max_fee_per_gas = Some(max_fee_per_gas + max_priority_fee_per_gas);
                } else {
                    let mut req = Eip1559TransactionRequest::new()
                        .to(tx.to().cloned().unwrap())
                        .data(tx.data().cloned().unwrap())
                        .max_priority_fee_per_gas(max_priority_fee_per_gas)
                        .max_fee_per_gas(max_fee_per_gas + max_priority_fee_per_gas);
                    if let Some(gas) = tx.gas() { req = req.gas(*gas); }
                    tx = TypedTransaction::Eip1559(req);
                }
            } else if let Ok(gas_price) = provider.get_gas_price().await {
                tx.set_gas_price(gas_price + max_priority_fee_per_gas);
            }

            log_message(&format!("🚀 Broadcasting TX to {} RPC endpoints (Flashbots/MEV)...", clients.len()));
            
            for client in &clients {
                let tx_clone = tx.clone();
                let client_clone = client.clone();
                tokio::spawn(async move {
                    match client_clone.send_transaction(tx_clone, None).await {
                        Ok(pending_tx) => {
                            log_message(&format!("✅ TX accepted by RPC! Hash: {:?}", pending_tx.tx_hash()));
                            match pending_tx.await {
                                Ok(Some(receipt)) => {
                                    if receipt.status == Some(U64::from(1)) {
                                        log_message(&format!("💎 Successfully minted! Block: {:?}", receipt.block_number.unwrap_or_default()));
                                    } else {
                                        log_message("❌ TX Reverted. Someone else likely beat us to it.");
                                    }
                                }
                                _ => log_message("❌ TX confirmation failed to retrieve."),
                            }
                        }
                        Err(e) => {
                            log_message(&format!("❌ TX send failed on this RPC: {:?}", e));
                        }
                    }
                });
            }

        } else {
            log_message("🔄 Challenge changed by network. Restarting on new challenge...");
        }
    }
}
