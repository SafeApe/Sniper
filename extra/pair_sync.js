const { MongoClient } = require('mongodb');
const fs = require('fs');
const path = require('path');
const { ethers } = require('ethers');

// Read config file
const config = JSON.parse(fs.readFileSync(path.join(__dirname, '../config.json'), 'utf8'));

// Minimal ERC20 ABI for getting token info
const ERC20_ABI = [
    "function name() view returns (string)",
    "function decimals() view returns (uint8)",
    "function owner() view returns (address)",
    "function getOwner() view returns (address)"
];

const PAIR_ABI = [
    "function token0() view returns (address)",
    "function token1() view returns (address)"
];

async function getTokenInfo(provider, tokenAddress) {
    try {
        const checksumAddress = ethers.getAddress(tokenAddress);
        const contract = new ethers.Contract(checksumAddress, ERC20_ABI, provider);
        const [name, decimals, owner,getOwner] = await Promise.all([
            contract.name().catch(() => "Unknown"),
            contract.decimals().catch(() => 18),
            contract.owner().catch(() => null),
            contract.getOwner().catch(() => null)
        ]);
        
        return {
            address: checksumAddress,
            name,
            owner: owner ? ethers.getAddress(owner) : getOwner ? ethers.getAddress(getOwner) : null,
            decimals,
            chain: 1, // Ethereum mainnet
            created_at: Date.now()
        };
    } catch (error) {
        console.error(`Error fetching token info for ${tokenAddress}:`, error);
        return null;
    }
}

async function syncPairs() {
    const client = await MongoClient.connect(config.db_uri);
    const provider = new ethers.JsonRpcProvider(config.networks.mainnet.rpc.replace('ws', 'http').replace('8546', '8545'));
    const chainId = await provider.getNetwork().then(network => network.chainId);
    try {
        console.log('Connected to MongoDB');
        
        const db = client.db(config.database);
        const pairsCollection = db.collection('pairs');
        const tokensCollection = db.collection('tokens');
        const tokenPairRelationsCollection = db.collection('token_pair_relations');
        
        // Read the cache file
        const cacheData = JSON.parse(fs.readFileSync(path.join(__dirname, '../cache/Ethereum_UniswapV2_cache.json'), 'utf8'));
        
        const BATCH_SIZE = 1000;
        let pairsBatch = [];
        let relationsBatch = [];
        let tokensBatch = [];
        let processedCount = 0;
        
        // Process pairs and tokens sequentially
        for (const [index, pool] of cacheData.pools.entries()) {
            const poolData = pool.UniswapV2;
            const pairAddress = ethers.getAddress(poolData.address);
            // print both tokens
            // console.log(`Processing pair ${index + 1} of ${cacheData.pools.length}: ${pairAddress}`);
            // write the progress in one line by refreshing the log
            process.stdout.write(`Processing pair ${index + 1} of ${cacheData.pools.length}: ${pairAddress}\r`);
            // Check if pair already exists
            const existingPair = await pairsCollection.findOne({ address: pairAddress });
            if (existingPair) {
                console.log(`Pair ${pairAddress} already exists, skipping...`);
                continue;
            }

            let token0Address = ethers.getAddress(poolData.token0);
            let token1Address = ethers.getAddress(poolData.token1);
            // if token0 or token1 address is more then 42, use pool address to fetch it
            if (token0Address.length > 42 || token1Address.length > 42) {
                const pairContract = new ethers.Contract(pairAddress, PAIR_ABI, provider);
                [token0Address, token1Address] = await Promise.all([
                    pairContract.token0().catch(() => null),
                    pairContract.token1().catch(() => null)
                ]);
            }

            // For first pool, add both tokens and set liq_token as null
            if (index === 0) {
                // Add both tokens if they don't exist
                for (const tokenAddress of [token0Address, token1Address]) {
                    const existingToken = await tokensCollection.findOne({ address: tokenAddress });
                    if (!existingToken) {
                        const tokenInfo = await getTokenInfo(provider, tokenAddress);
                        if (tokenInfo) {
                            tokensBatch.push(tokenInfo);
                        }
                        tokenInfo.chain = 1;
                    }
                }

                // Add pair with no liq_token
                const pair = {
                    address: pairAddress,
                    token1: token0Address,
                    token2: token1Address,
                    pool_version: '2',
                    dex: 'Uniswap-V2',
                    liq_token: token1Address,
                    created_at: Date.now()
                };

                pairsBatch.push(pair);
                
                // Create token-pair relationships
                relationsBatch.push({
                    token_address: token0Address,
                    pair_address: pairAddress,
                    created_at: Math.floor(Date.now() / 1000)
                });
                
                relationsBatch.push({
                    token_address: token1Address,
                    pair_address: pairAddress,
                    created_at: Math.floor(Date.now() / 1000)
                });
            } else {
                // Check which token exists in db (that's our liq token)
                const token0Exists = await tokensCollection.findOne({ address: token0Address });
                const token1Exists = await tokensCollection.findOne({ address: token1Address });
                
                let liqToken, newToken;
                
                if (token0Exists && !token1Exists) {
                    liqToken = token0Address;
                    newToken = token1Address;
                } else if (!token0Exists && token1Exists) {
                    liqToken = token1Address;
                    newToken = token0Address;
                } else if (token0Exists && token1Exists) {
                    // Both tokens exist, find which one was added first
                    const tokens = await tokensCollection.find({
                        address: { $in: [token0Address, token1Address] }
                    }).sort({ created_at: 1 }).toArray();
                    // console.log("Order: ",tokens);
                    if (tokens.length === 2) {
                        liqToken = tokens[0].address; // First token is the liq token
                        newToken = tokens[1].address;
                        
                        // Create token-pair relationships
                        relationsBatch.push({
                            token_address: liqToken,
                            pair_address: pairAddress,
                            created_at: Math.floor(Date.now() / 1000)
                        });
                        
                        relationsBatch.push({
                            token_address: newToken,
                            pair_address: pairAddress,
                            created_at: Math.floor(Date.now() / 1000)
                        });
                    } else {
                        // find each one separately
                        const token0index = await tokensCollection.findOne({ address: token0Address });
                        const token1index = await tokensCollection.findOne({ address: token1Address });
                        if (token0index.created_at < token1index.created_at) {
                            liqToken = token0Address;
                            newToken = token1Address;
                        } else {
                            liqToken = token1Address;
                            newToken = token0Address;
                        }
                    }
                } else {
                    // get token creation time from dexscreen api
                    
                    // fetch via blockchain
                    // console.log(`Token0 : ${token0Address}\nToken1 : ${token1Address}`);
                    console.log(`Skipping pair ${pairAddress} - no tokens found`);
                    // process.exit(1);
                    // continue;
                }

                // Add the new token
                const tokenInfo = await getTokenInfo(provider, newToken);
                if (tokenInfo) {
                    tokensBatch.push(tokenInfo);
                }

                // Add the pair with liq_token
                const pair = {
                    address: pairAddress,
                    token1: token0Address,
                    token2: token1Address,
                    pool_version: 'v2',
                    dex: 'UniswapV2',
                    liq_token: liqToken,
                    created_at: Date.now()
                };

                pairsBatch.push(pair);
            }
            
            // console.log(`Processed pair ${pairAddress}`);
            
            // Insert batches when they reach the size limit
            if (pairsBatch.length >= BATCH_SIZE) {
                if (pairsBatch.length > 0) {
                    await pairsCollection.insertMany(pairsBatch);
                    console.log(`Inserted ${pairsBatch.length} pairs`);
                    pairsBatch = [];
                }
                if (relationsBatch.length > 0) {
                    await tokenPairRelationsCollection.insertMany(relationsBatch);
                    console.log(`Inserted ${relationsBatch.length} relations`);
                    relationsBatch = [];
                }
                if (tokensBatch.length > 0) {
                    await tokensCollection.insertMany(tokensBatch);
                    console.log(`Inserted ${tokensBatch.length} tokens`);
                    tokensBatch = [];
                }
            }
            
            processedCount++;
        }
        
        // Insert any remaining items in the batches
        if (pairsBatch.length > 0) {
            await pairsCollection.insertMany(pairsBatch);
            console.log(`Inserted final ${pairsBatch.length} pairs`);
        }
        if (relationsBatch.length > 0) {
            await tokenPairRelationsCollection.insertMany(relationsBatch);
            console.log(`Inserted final ${relationsBatch.length} relations`);
        }
        if (tokensBatch.length > 0) {
            await tokensCollection.insertMany(tokensBatch);
            console.log(`Inserted final ${tokensBatch.length} tokens`);
        }
        
        // Create indexes
        await pairsCollection.createIndex({ 'token1': 1 });
        await pairsCollection.createIndex({ 'token2': 1 });
        await pairsCollection.createIndex({ 'address': 1 }, { unique: true });
        await pairsCollection.createIndex({ 'liq_token': 1 });
        
        await tokensCollection.createIndex({ 'address': 1 }, { unique: true });
        
        await tokenPairRelationsCollection.createIndex({ 'token_address': 1 });
        await tokenPairRelationsCollection.createIndex({ 'pair_address': 1 });
        
        console.log(`Pair and token synchronization completed. Processed ${processedCount} pairs`);
    } catch (error) {
        console.error('Error syncing pairs:', error);
    } finally {
        await client.close();
    }
}

// Run the sync
syncPairs().catch(console.error);