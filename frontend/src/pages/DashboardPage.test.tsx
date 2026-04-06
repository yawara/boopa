import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { AppProviders } from "../app/AppProviders";
import { createTestStore } from "../app/store";
import { DashboardPage } from "./DashboardPage";

function mockApi() {
  const base = `${window.location.origin}/api`;
  const responses = new Map<string, Response>([
    [
      `${base}/distros`,
      new Response(
        JSON.stringify({
          selected: "ubuntu",
          distros: [
            { id: "ubuntu", label: "Ubuntu" },
            { id: "fedora", label: "Fedora" },
            { id: "arch", label: "Arch Linux" },
          ],
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    ],
    [
      `${base}/dhcp?distro=ubuntu`,
      new Response(
        JSON.stringify({
          selected: "ubuntu",
          bios: {
            bootFilename: "ubuntu/bios/lpxelinux.0",
            nextServer: "set to the boopa host IP",
            architecture: "x86 BIOS",
            notes: ["bios note"],
            options: [{ key: "filename", value: "ubuntu/bios/lpxelinux.0", description: "desc" }],
          },
          uefi: {
            bootFilename: "ubuntu/uefi/grubx64.efi",
            nextServer: "set to the boopa host IP",
            architecture: "x86_64 UEFI",
            notes: ["uefi note"],
            options: [{ key: "filename", value: "ubuntu/uefi/grubx64.efi", description: "desc" }],
          },
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    ],
    [
      `${base}/cache`,
      new Response(
        JSON.stringify({
          selected: "ubuntu",
          entries: [
            {
              distroId: "ubuntu",
              bootMode: "bios",
              logicalName: "kernel",
              sourceUrl: "https://example.test/kernel",
              relativePath: "ubuntu/bios/kernel",
              localPath: "/tmp/ubuntu/bios/kernel",
              status: "cached",
              lastSyncedAt: null,
            },
          ],
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    ],
  ]);

  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const request = input instanceof Request ? input : new Request(input, init);
    const url = request.url;
    if (request.method === "PUT" && url.endsWith("/api/selection")) {
      return new Response(JSON.stringify({ selected: "fedora" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }

    const response = responses.get(url);
    if (!response) {
      throw new Error(`Unexpected request: ${url}`);
    }

    return response.clone();
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

    await userEvent.selectOptions(screen.getByLabelText("Distro"), "fedora");

    await waitFor(() => {
      expect(fetchMock.mock.calls.some(([input]) => {
        const request = input instanceof Request ? input : new Request(input);
        return request.method === "PUT" && request.url.endsWith("/api/selection");
      })).toBe(true);
    });
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
