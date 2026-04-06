import os from "node:os";
import path from "node:path";

export const e2eApiPort = 38181;
export const e2eTftpPort = 36970;
export const e2eVitePort = 4174;
export const e2eDataDir = path.join(os.tmpdir(), "boopa-frontend-e2e");
export const e2eBackendOrigin = `http://127.0.0.1:${e2eApiPort}`;
export const e2eBaseUrl = `http://127.0.0.1:${e2eVitePort}`;

export function buildBoopaEnv() {
  return {
    BOOPA_API_BIND: `127.0.0.1:${e2eApiPort}`,
    BOOPA_TFTP_BIND: `127.0.0.1:${e2eTftpPort}`,
    BOOPA_TFTP_ADVERTISE_ADDR: `127.0.0.1:${e2eTftpPort}`,
    BOOPA_DATA_DIR: e2eDataDir,
  };
}

export function buildViteEnv() {
  return {
    BOOPA_DEV_BACKEND: e2eBackendOrigin,
  };
}
