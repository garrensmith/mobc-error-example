Mobc reproduction error

To generate the error:

    ```
    docker compose up -d postgres
    ``

Then in the overload folder:

```
    npx prisma migrate dev
    node seed2.js
```

Then in the root folder

```
cargo run --release
```

In another terminal

```
ab -v 4 -c 200  -t 120 http://127.0.0.1:4000/

```

Once it has completed. Wait a minute. And run it again. It should log less connections to the database, or none at all.
