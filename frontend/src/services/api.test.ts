import { createTestStore } from "../app/store";
import { networkBootApi } from "./api";

describe("networkBootApi", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("loads distros from the backend contract", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          selected: "ubuntu",
          distros: [
            { id: "ubuntu", label: "Ubuntu" },
            { id: "fedora", label: "Fedora" },
          ],
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      ),
    );

    vi.stubGlobal("fetch", fetchMock);
    const store = createTestStore();
    const result = await store.dispatch(networkBootApi.endpoints.getDistros.initiate());
    const [request] = fetchMock.mock.calls[0] as [Request];

    expect(result.data?.selected).toBe("ubuntu");
    expect(request.url).toContain("/api/distros");
  });

  it("sends selection updates as the backend expects", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ selected: "fedora" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    vi.stubGlobal("fetch", fetchMock);
    const store = createTestStore();
    await store.dispatch(networkBootApi.endpoints.setSelection.initiate("fedora"));

    const [request] = fetchMock.mock.calls[0] as [Request];
    expect(request.method).toBe("PUT");
    await expect(request.text()).resolves.toBe(JSON.stringify({ distro: "fedora" }));
  });
});
