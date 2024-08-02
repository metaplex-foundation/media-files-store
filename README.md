# Storage Service

A service that is used to download, post-process and provide NFT media assets

Design page: https://github.com/adm-metaex/doc/blob/main/projects/metagrid/rollup/storage_service.md

The flow of assets downloading is following:

```
┌------┐                                                  ╒==========╕
|Solana|                                                  | INTERNET |
└------┘                                                  ╘==========╛
    │                                                          Λ │
    │1            + - Media service - - - - - - - - - - - - -  │ │6 - - - +
    V                               ⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯    5       │ V             
╔════════╗    3   | ┌───────┐      ⎛⎞              ⎞ ---->   ┌──────────┐ | 7    ⎯⎯⎯⎯⎯⎯
║        ║ -------> |fetcher| ---> ⎜⎟download tasks⎟  ---> ┌─┴────────┐ |---->  /      \
║        ║        | └───────┘      ⎝⎠⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎠ ----> |  worker  |─┘ |    │\⎯⎯⎯⎯⎯⎯/│
║  Das   ║                                                 └──────────┘        │        │
║  node  ║        |                 ⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯         |           |    │   S3   │
║        ║    9     ┌─────────┐    ⎛⎞                ⎞        |8                \______/
║        ║ <------- |submitter| <- ⎜⎟download results⎟ <------┘           |
╚════════╝          └─────────┘    ⎝⎠⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎠ 
  │  Λ  │         |                                                       |
 2│ 4│  │⒑
 ⎯V⎯⎯│⎯⎯V⎯⎯       + - - - - - - - - - - - - - - - - - - - - - - - - - - - +
(⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯)
| Rocks DB |
╰──────────╯    
```

Description:

1. The flow starts at the DAS node, when it retrieves another rollup creation from the Solana.
  The DAS note downloads corresponding rollup JSON ands parses it, and saves to the storage (RocksDB)
2. After parsing and persisting the rollup data, the DAS note takes all the asset URLs from the rollup
  and pushes them to RocksDB-based collection, which serves as a queue for these asset "URLs to be downloaded".
3. Then the Media service comes to the play. Using GRPC protocol, the media service fetches a potion
  of asset URLs for downloading.
4. These URLs are retrieved from the RocksDB-based "URLs to download" queue we have mentioned above.
5. On the Media service side these URLs are divided among multiple workers via
  [async_channel](https://docs.rs/async-channel/latest/async_channel/)
6. A worker takes next URL from the queue, downloads the asset, and if the asset is a picture,
  then resizes it to fit 400×400 pixels bounding box.
7. Then this downloaded and resized image is persisted into an S3 compatible storage (MINIO).
  A keccak256 hash of the asset URL serves as the object key.
8. The result of this whole downloading and resizing  process is pushed to the results queue.
9. Download results are aggregated into batches and sent to the DAS node.
10. DAS node removes processed URLs from the "URLs to download queue" and adds information
  about newly fetched asset to the "asset URLs" table.
  Later when client query NFTs that contains asset downloaded by the media service,
  preview asset URLs for these assert will be replaced with URLs pointing to the media service.

TODO: replace "rollup" with new name

## Image storing

After downloading an image, we resize images to 400×400 bounding box and store it into S3-compatible storage.

We use keccak256 hash of the asset URL as the S3 object key for the stored image.

## Running locally

To run locally you need:

1. Run local Minio
2. Select proper config that matches your local environment
3. run the application

Note, that for fetching new URLs for downloading, you also need to have a DAS node (utility-chaing)
`ingester` service running.

---

### Running MINIO locally

Make sure you've downloaded MINIO from https://min.io/download

Run Minio locally:
```
MINIO_ROOT_USER=admin MINIO_ROOT_PASSWORD=password ./minio server $HOME/dev/data/minio --console-address ":9001"
```

---

### Run config

Configs are taken from `config/default.toml` + `config/${ENV}.toml` files, where the ENV is defined by `RUN_ENV` system variable.
E.g. if you want to make a custom run config, you can create a file `config/my_conf.toml` and make `export RUN_ENV=my_conf`.

S3 configs (also used by Minio) can alternative be set in a traditional AWS way:

```sh
export AWS_ENDPOINT_URL="http://127.0.0.1:9000"
export AWS_ACCESS_KEY_ID=admin
export AWS_SECRET_ACCESS_KEY=password
export AWS_DEFAULT_REGION=us-east-1
```

### Launching the application

```sh
RUN_ENV=my_conf cargo run
```