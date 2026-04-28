const { MongoClient } = require('mongodb');
const config = require('../config.json');

async function migratePairs() {
    const client = await MongoClient.connect(config.db_uri);
    try {
        console.log('Connected to MongoDB');
        
        const db = client.db(config.database);
        const pairsCollection = db.collection('pairs');
        const tokensCollection = db.collection('tokens');
        const tokenPairRelationsCollection = db.collection('token_pair_relations');

        // First, migrate all token pairs to relations
        const tokens = await tokensCollection.find({ pairs: { $exists: true } }).toArray();
        console.log(`Found ${tokens.length} tokens with pairs to migrate`);

        let relationsCreated = 0;

        for (const token of tokens) {
            if (!token.pairs || !Array.isArray(token.pairs)) continue;

            const tokenAddress = token.address;
            const timestamp = token.created_at || Math.floor(Date.now() / 1000);

            // Create relations for each pair
            const relations = token.pairs.map(pairAddress => ({
                token_address: tokenAddress,
                pair_address: pairAddress,
                created_at: timestamp
            }));

            if (relations.length > 0) {
                await tokenPairRelationsCollection.insertMany(relations);
                relationsCreated += relations.length;
                
                // Remove pairs field from this token
                await tokensCollection.updateOne(
                    { _id: token._id },
                    { $unset: { pairs: "" } }
                );
            }

            if (relationsCreated % 1000 === 0) {
                console.log(`Created ${relationsCreated} relations...`);
            }
        }

        // Create indexes for the new collection
        await tokenPairRelationsCollection.createIndex({ 'token_address': 1 });
        await tokenPairRelationsCollection.createIndex({ 'pair_address': 1 });
        
        // Double check and remove any remaining pairs fields
        const result = await tokensCollection.updateMany(
            { pairs: { $exists: true } },
            { $unset: { pairs: "" } }
        );

        console.log(`Migration completed successfully!`);
        console.log(`Created ${relationsCreated} token-pair relations`);
        console.log(`Cleaned up pairs field from ${result.modifiedCount} remaining tokens`);

    } catch (error) {
        console.error('Error during migration:', error);
    } finally {
        await client.close();
    }
}

// Run the migration
migratePairs().catch(console.error);