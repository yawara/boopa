import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { AppProviders } from "../app/AppProviders";
import { createTestStore } from "../app/store";
import { DashboardPage } from "./DashboardPage";

function buildRenderedYaml(hostname: string) {
  return `#cloud-config
autoinstall:
  version: 1
  identity:
    hostname: ${hostname}
    username: ubuntu
    password: $1$mock$passwordhash
`;
}

function mockApi(options?: { selected?: "ubuntu" | "fedora" | "arch" }) {
  const base = `${window.location.origin}/api`;
  let selected = options?.selected ?? "ubuntu";
  let autoinstall = {
    config: {
      hostname: "boopa-ubuntu",
      username: "ubuntu",
      locale: "en_US.UTF-8",
      keyboardLayout: "us",
      timezone: "UTC",
      storageLayout: "direct",
      installOpenSsh: true,
      allowPasswordAuth: true,
      authorizedKeys: [],
      packages: ["curl"],
    },
    renderedYaml: buildRenderedYaml("boopa-ubuntu"),
    hasPassword: true,
  };

  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const request = input instanceof Request ? input : new Request(input, init);
    const url = request.url;

    if (request.method === "GET" && url.endsWith("/api/distros")) {
      return new Response(
        JSON.stringify({
          selected,
          distros: [
            { id: "ubuntu", label: "Ubuntu" },
            { id: "fedora", label: "Fedora" },
            { id: "arch", label: "Arch Linux" },
          ],
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      );
    }

    if (request.method === "GET" && url.endsWith(`/api/dhcp?distro=${selected}`)) {
      return new Response(
        JSON.stringify({
          selected,
          bios: {
            bootFilename: `${selected}/bios/lpxelinux.0`,
            nextServer: "set to the boopa host IP",
            architecture: "x86 BIOS",
            notes: ["bios note"],
            options: [{ key: "filename", value: `${selected}/bios/lpxelinux.0`, description: "desc" }],
          },
          uefi: {
            bootFilename: `${selected}/uefi/grubx64.efi`,
            nextServer: "set to the boopa host IP",
            architecture: "x86_64 UEFI",
            notes: ["uefi note"],
            options: [{ key: "filename", value: `${selected}/uefi/grubx64.efi`, description: "desc" }],
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      );
    }

    if (request.method === "GET" && url.endsWith("/api/cache")) {
      return new Response(
        JSON.stringify({
          selected,
          entries: [
            {
              distroId: selected,
              bootMode: "bios",
              logicalName: "kernel",
              sourceUrl: "https://example.test/kernel",
              relativePath: `${selected}/bios/kernel`,
              localPath: `/tmp/${selected}/bios/kernel`,
              status: "cached",
              lastSyncedAt: null,
            },
          ],
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      );
    }

    if (request.method === "GET" && url.endsWith("/api/autoinstall/ubuntu")) {
      return new Response(JSON.stringify(autoinstall), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }

    if (request.method === "PUT" && url.endsWith("/api/selection")) {
      const body = await request.json();
      selected = body.distro;
      return new Response(JSON.stringify({ selected }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }

    if (request.method === "PUT" && url.endsWith("/api/autoinstall/ubuntu")) {
      const body = await request.json();
      autoinstall = {
        config: {
          hostname: body.hostname,
          username: body.username,
          locale: body.locale,
          keyboardLayout: body.keyboardLayout,
          timezone: body.timezone,
          storageLayout: body.storageLayout,
          installOpenSsh: body.installOpenSsh,
          allowPasswordAuth: body.allowPasswordAuth,
          authorizedKeys: body.authorizedKeys,
          packages: body.packages,
        },
        renderedYaml: buildRenderedYaml(body.hostname),
        hasPassword: true,
      };

      return new Response(JSON.stringify(autoinstall), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }

    throw new Error(`Unexpected request: ${request.method} ${url}`);
  });

  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

describe("DashboardPage", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("renders the dashboard and allows distro selection", async () => {
    const fetchMock = mockApi();
    const store = createTestStore();
    render(
      <AppProviders store={store}>
        <DashboardPage />
      </AppProviders>,
    );

    expect(await screen.findByText("boopa")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "boopa logo" })).toBeInTheDocument();
    expect(await screen.findByText("Asset readiness for ubuntu")).toBeInTheDocument();
    expect(await screen.findByText("Manual settings for ubuntu")).toBeInTheDocument();
    expect(await screen.findByText("Ubuntu autoinstall defaults")).toBeInTheDocument();

    await userEvent.selectOptions(screen.getByLabelText("Distro"), "fedora");

    await waitFor(() => {
      expect(fetchMock.mock.calls.some(([input]) => {
        const request = input instanceof Request ? input : new Request(input);
        return request.method === "PUT" && request.url.endsWith("/api/selection");
      })).toBe(true);
    });
  });

  it("validates and saves ubuntu autoinstall config", async () => {
    const fetchMock = mockApi();
    render(
      <AppProviders store={createTestStore()}>
        <DashboardPage />
      </AppProviders>,
    );

    const hostname = await screen.findByLabelText("Hostname");
    const saveButton = await screen.findByRole("button", { name: "Save Autoinstall Config" });

    await userEvent.clear(hostname);
    expect(await screen.findByText(/Hostname must be 1-63 characters/)).toBeInTheDocument();
    expect(saveButton).toBeDisabled();

    await userEvent.type(hostname, "lab-node");
    await waitFor(() => {
      expect(saveButton).toBeEnabled();
    });

    await userEvent.click(saveButton);

    await waitFor(() => {
      expect(fetchMock.mock.calls.some(([input]) => {
        const request = input instanceof Request ? input : new Request(input);
        return request.method === "PUT" && request.url.endsWith("/api/autoinstall/ubuntu");
      })).toBe(true);
    });

    expect(await screen.findByText(/hostname: lab-node/)).toBeInTheDocument();
  });

  it("hides the autoinstall panel when ubuntu is not selected", async () => {
    mockApi({ selected: "fedora" });
    render(
      <AppProviders store={createTestStore()}>
        <DashboardPage />
      </AppProviders>,
    );

    expect(await screen.findByText("Asset readiness for fedora")).toBeInTheDocument();
    expect(screen.queryByText("Ubuntu autoinstall defaults")).not.toBeInTheDocument();
  });

  it("shows the hero immediately while the dashboard data is still loading", () => {
    const pendingFetch = vi.fn(
      () =>
        new Promise<Response>(() => {
          // Leave the request pending to keep the page in its initial loading state.
        }),
    );

    vi.stubGlobal("fetch", pendingFetch);

    render(
      <AppProviders store={createTestStore()}>
        <DashboardPage />
      </AppProviders>,
    );

    expect(screen.getByRole("img", { name: "boopa logo" })).toBeInTheDocument();
    expect(screen.getByText("Loading dashboard...")).toBeInTheDocument();
  });
});
