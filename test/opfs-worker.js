const mem = {};

self.onmessage = (event) =>
  Promise.resolve().then((_) => {
    const { data } = event;
    const {
      dirName,
      payload,
      fileName,
    } = data;

    let handle;

    return navigator.storage.getDirectory()
      .then((rt) => {
        return rt.getDirectoryHandle(dirName, { create: true })
          .then((dir) => [rt, dir]);
      })
      .then((pair) => {
        const [rt, dir] = pair;
        return dir.getFileHandle(fileName, { create: true })
          .then((file) => [rt, dir, file]);
      })
      .then((rdf) => {
        const [rt, dir, file] = rdf;
        return file.createSyncAccessHandle()
          .then((hndl) => [rt, dir, file, hndl]);
      })
      .then((rdfh) => {
        const [rt, dir, file, hndl] = rdfh;
        handle = hndl;

        hndl.truncate(0);
        hndl.write(
          new Uint8Array([
            0x28,
            0xb5,
            0x2f,
            0xfd,

            0x04,
            0x58,
            0x21,
            0x00,

            0x00,
            0x68,
            0x65,
            0x6c,

            0x6f,
            0xbd,
            0x1b,
            0x8d,

            0xab,
          ]),
          { at: 0 },
        );
        hndl.flush();

        const toffset = 0x20;

        const dec = new TextDecoder();

        return WebAssembly.instantiateStreaming(
          fetch(new URL("./rs_zstdcat4opfs.wasm", import.meta.url)),
          {
            env: {
              bytes_sink: (ptr, len) => {
                const view = new Uint8Array(
                  mem.wasmMem.buffer,
                  ptr,
                  len,
                );
                const decoded = dec.decode(view);
                self.postMessage({
                  ok: true,
                  done: false,
                  msg: { decoded },
                });
                return len;
              },

              opfs_read: (ptr, len, offset) => {
                if (!mem.wasmMem) return -1;

                const view = new Uint8Array(
                  mem.wasmMem.buffer,
                  ptr,
                  len,
                );
                return hndl.read(view, { at: offset });
              },
            },
          },
        )
          .then((i) => [rt, dir, file, hndl, i]);
      })
      .then((rdfhi) => {
        const [rt, dir, file, hndl, wasm] = rdfhi;

        const { instance } = wasm;
        const { exports } = instance;
        const { e_io_opfs2zcat2sink } = exports;

        mem.wasmMem = exports.memory;

        const cnt = e_io_opfs2zcat2sink(1048575);

        const ok = 4 == cnt;
        if (ok) {
          return self.postMessage({
            ok,
            done: true,
          });
        }

        self.postMessage({
          ok,
          msg: new Error(`expected: 4, got: ${cnt}`),
          done: true,
        });
      })
      .catch((err) => {
        self.postMessage({
          ok: false,
          msg: `failure: ${err}`,
          done: true,
        });
      })
      .finally((_) => {
        if (!handle) return;
        handle.close();
      });
  });
