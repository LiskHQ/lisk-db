const { Database } = require("../main");
const crypto = require('crypto');

/*const getRandomBytes = (num = 32) => crypto.randomBytes(num);

(async () => {
    const db = new Database('.tmp', { readonly: false });

    const keys = [];
    for (let i = 0; i < 1000; i++) {
        const key = getRandomBytes();
        await db.set(key, getRandomBytes(100))
        keys.push(key);
    }

    for (let i = 0; i < 1000; i++) {
        console.log('key', keys.length)
        console.time('get');
        const stream = await db.iterate({ limit: 1000 });
        const blockIDs = await new Promise((resolve, reject) => {
            const ids = [];
            stream
                .on('data', ({ value }) => {
                    ids.push(value);
                })
                .on('error', error => {
                    reject(error);
                })
                .on('end', () => {
                    resolve(ids);
                });
        });
        console.log('*'.repeat(100));
        console.log(blockIDs.length)
        console.timeEnd('get');
    }
    console.log('done')

    await db.close();

})()*/

(async () => {
    const db = new Database('./backup', { readonly: true })
    const DB_KEY_BLOCKS_ID = 'blocks:id';
    console.time('performance');

    const blocksStream = db.createReadStream({
        gte: Buffer.from(`${DB_KEY_BLOCKS_ID}:${Buffer.alloc(32, 0).toString('binary')}`),
        lte: Buffer.from(`${DB_KEY_BLOCKS_ID}:${Buffer.alloc(32, 255).toString('binary')}`),
    });

    let i = 0;

    await new Promise((resolve, reject) => {
        blocksStream
            .on('data', ({ value }) => {
                // console.log(value)
            })
            .on('error', error => {
                reject(error);
            })
            .on('end', () => {
                resolve();
            });
    });
    console.timeEnd('performance');
})()