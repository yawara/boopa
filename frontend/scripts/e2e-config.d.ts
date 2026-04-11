declare module "./e2e-config.mjs" {
  export const e2eApiPort: number;
  export const e2eTftpPort: number;
  export const e2eVitePort: number;
  export const e2eDataDir: string;
  export const e2eBackendOrigin: string;
  export const e2eBaseUrl: string;

  export function buildBoopaEnv(): Record<string, string>;
  export function buildViteEnv(): Record<string, string>;
}
