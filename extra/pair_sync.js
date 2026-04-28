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

        // First, collect all pairs and tokens to check
        const pairsToCheck = [];
        const tokensToCheck = new Set();
        
        for (const [index, pool] of cacheData.pools.entries()) {
            const poolData = pool.UniswapV2;
            const pairAddress = ethers.getAddress(poolData.address);
            const token0Address = ethers.getAddress(poolData.token0);
            const token1Address = ethers.getAddress(poolData.token1);
            
            pairsToCheck.push({
                address: pairAddress,
                token0: token0Address,
                token1: token1Address
            });
            
            tokensToCheck.add(token0Address);
            tokensToCheck.add(token1Address);
        }

        // Bulk check existing pairs
        const existingPairs = await pairsCollection.find({
            $or: pairsToCheck.map(pair => ({
                $or: [
                    { token1: pair.token0, token2: pair.token1 },
                    { token1: pair.token1, token2: pair.token0 }
                ]
            }))
        }).toArray();

        const existingPairMap = new Map(
            existingPairs.map(pair => [
                `${pair.token1}_${pair.token2}`,
                true
            ])
        );

        // Bulk check existing tokens
        const existingTokens = await tokensCollection.find({
            address: { $in: Array.from(tokensToCheck) }
        }).toArray();

        const existingTokenMap = new Map(
            existingTokens.map(token => [token.address, true])
        );

        // Now process pairs that don't exist
        for (const pair of pairsToCheck) {
            const pairKey1 = `${pair.token0}_${pair.token1}`;
            const pairKey2 = `${pair.token1}_${pair.token0}`;
            
            if (!existingPairMap.has(pairKey1) && !existingPairMap.has(pairKey2)) {
                // Check and add tokens if they don't exist
                for (const tokenAddress of [pair.token0, pair.token1]) {
                    if (!existingTokenMap.has(tokenAddress)) {
                        const tokenInfo = await getTokenInfo(provider, tokenAddress);
                        if (tokenInfo) {
                            tokenInfo.chain = 1;
                            tokensBatch.push(tokenInfo);
                            existingTokenMap.set(tokenAddress, true);
                        }
                    }
                }

                // Add new pair
                const newPair = {
                    address: pair.address,
                    token1: pair.token0,
                    token2: pair.token1,
                    pool_version: "v2",
                    dex: 'Uniswap-V2',
                    liq_token: pair.token1,
                    created_at: Date.now()
                };

                pairsBatch.push(newPair);
                
                // Add relations for both tokens
                relationsBatch.push({
                    token_address: pair.token0,
                    pair_address: pair.address,
                    created_at: Math.floor(Date.now() / 1000)
                });
                
                relationsBatch.push({
                    token_address: pair.token1,
                    pair_address: pair.address,
                    created_at: Math.floor(Date.now() / 1000)
                });

                processedCount++;
            }

            // Batch inserts
            if (pairsBatch.length >= BATCH_SIZE) {
                if (tokensBatch.length > 0) {
                    await tokensCollection.insertMany(tokensBatch);
                    console.log(`Inserted ${tokensBatch.length} tokens`);
                    tokensBatch = [];
                }
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
            }
        }
        
        // Insert remaining batches
        if (tokensBatch.length > 0) {
            await tokensCollection.insertMany(tokensBatch);
            console.log(`Inserted final ${tokensBatch.length} tokens`);
        }
        if (pairsBatch.length > 0) {
            await pairsCollection.insertMany(pairsBatch);
            console.log(`Inserted final ${pairsBatch.length} pairs`);
        }
        if (relationsBatch.length > 0) {
            await tokenPairRelationsCollection.insertMany(relationsBatch);
            console.log(`Inserted final ${relationsBatch.length} relations`);
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