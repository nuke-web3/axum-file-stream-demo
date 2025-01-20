# SS streaming file upload

A minimal streaming files example

You can run it with `cargo run --bin example-axum`, and then visit the documentation at `http://localhost:3000`.

## TODO

- [ ] state.rs to have some appstate?
- [ ] upload multiple files
- [ ] stream file(s) from GET to axum
- [ ] handle auto deletion
  - [ ] on successful download
  - [ ] on date experation (spawn a task for tokio::time)

## Goal

> From https://github.com/dpasirst/shield-server/issues/82

---

As an end user, I need SecretShield to support shares that are large (megabytes in size) so that I can save and recover SecretSecrets that are large (megabytes in size).

**Assumptions:**

    * Instead of the shares being sent to the Guardians (or on recovery, back to the Secret Owner) via the XMPP message server, the share data would be uploaded to the shield-server which would provide an identifier back to the client. The identifier would be transmitted as part of the share distribution to the Guardian (or recovery to the Secret Owner) which can then use that identifier to download the data from the server.
    * Share issuance is double ratchet encrypted per Guardian which means that there would be a unique file uploaded per Guardian. E.g. a 5MB secret sent to 5 guardians, would result in 5 unique 5MB uploads (total 25MB).

**Acceptance Criteria:**

    * The server should have a REST GET endpoint to query the maximum file size. The result should return JSON indicating the configured (or defaulted) maximum size in bytes.

      * the default should be 10MB

    * The server should support data (file) uploads, as a blob of binary data of any arbitrary size limited by a configurable environment variable.

    * The server should be very careful in using memory, and stream the file to the predetermined destination for server-side persistence.

    * The server should support data (file) download.

    * The server should be very careful in using memory, and stream the file to the client from the predetermined destination for server-side persistence.

    * All server end-points must be implemented using AIDE+AXUM and have valid OpenAPI generated that can be used by:

      * iOS (existing openapi impl)
      * Android (existing openapi impl)
      * Rust
      * Typescript (existing openapi impl)

    * All server transactions must come from authenticated clients.

    * The client must **not** be involved or in anyway control the name or identifier of the file. This must be generated and issued by the shield-server.

    * Upon successfully streaming the file to the client the server should delete the file.

      * if the server detected a broken connection (incomplete download) then it should not delete the file

    * If the file is not downloaded within a configurable (or default) amount of time in seconds, then the server should delete the data.

      * the default should be 14 days

    * The persistence must be through an abstration layer (such as a trait) that can be replaced in the future. For example, today it may write the file directly to disk but in the future it may store it to a bucket like S3.

    * Pre/Well Defined Errors with open telemetry events should be recorded with unique messages with assigned codes returned for:

      * user is not authenticated
      * a file upload exceeding the maximum size
      * server fails to write the file
      * the requested file no longer exists on the server
      * the server detects a failure/corruption/early termination in the file upload or download

**Question:** Should the double ratchet header be in the uploaded file or should it be in the share distribution XMPP message? It might be easier if the data/file were just an encrypted blob and the ratchet header were with the distribution message. But it would require the client to ephemerally persist that information until the file is successfully downloaded (or no longer available on the server).

**Engineering Notes:** The server may (optionally) handle this statelessly (no need to store state in the database). This means the server will not track the client that uploaded the data or the client that downloaded the data other than in the event logs. Metadata such as the creation time stored with the uploaded data may be used for expiration/deletion.

AIDE may impose additional restrictions on the implementation, both in Rust and in what the clients support. It may be worth defining the endpoint to generate the OpenAPI and test that it can be used with the other clients. Sometimes it will work with some but not all of those clients and some changes may be required.

Some of the errors already have handling that will come for free between Axum and the handle-errors crate.

@nuke-web3 has a Repo: https://github.com/nuke-web3/axum-file-stream-demo with some example code to get started. At this time, it does not handle AIDE OpenAPI generation for the file endpoint or streaming downloads.

The importance of having the server create the file name and identifier is for security. This prevents a client from being able to manipulate the server based on a filename (or file name path) and prevents a client from being able to logically guess a file to download.

While not relevant for the server, this will cause the client to encrypt the data upload first and then derive the next key (rather than rotate to a completely new one) for encrypting the message that will be sent to the Guardian. This also means the guardian will receive the results "out of order" in that the derived message will be received and decrypted first with the information necessary to download and then decrypt the data. While we actively try to avoid this out-of-order scenario and prefer full key replacement (completely new key), we currently allow for this, it is part of the client unit tests and it should not be a problem.
