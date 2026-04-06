import { createTheme, rgba } from "@mantine/core";

const slate = [
  "#f2f4f7",
  "#e2e7ee",
  "#c8d1dc",
  "#a4b2c1",
  "#8295a8",
  "#687e92",
  "#587084",
  "#4a5c6e",
  "#404e5d",
  "#37424f",
] as const;

const sand = [
  "#fff8ef",
  "#fff0dc",
  "#f9ddb4",
  "#f2c788",
  "#ecb45f",
  "#e6a13b",
  "#dd922b",
  "#c47d1f",
  "#ad6d17",
  "#955b0b",
] as const;

const accent = [
  "#fff1ea",
  "#fddfd2",
  "#f6baa2",
  "#ef9270",
  "#e97346",
  "#e45d2b",
  "#e0511c",
  "#c84310",
  "#b23a0c",
  "#9b2f05",
] as const;

export const boopaTheme = createTheme({
  primaryColor: "boopaAccent",
  defaultRadius: "xl",
  fontFamily: '"IBM Plex Sans", "Segoe UI", sans-serif',
  headings: {
    fontFamily: '"IBM Plex Sans", "Segoe UI", sans-serif',
    fontWeight: "700",
  },
  colors: {
    slate,
    sand,
    boopaAccent: accent,
  },
  black: "#24303b",
  white: "#fffdf9",
  defaultGradient: {
    from: "boopaAccent.5",
    to: "boopaAccent.7",
    deg: 135,
  },
  shadows: {
    sm: `0 18px 40px ${rgba("#274152", 0.1)}`,
    md: `0 24px 54px ${rgba("#274152", 0.14)}`,
    xl: `0 30px 72px ${rgba("#274152", 0.18)}`,
  },
  components: {
    Paper: {
      defaultProps: {
        radius: "32px",
        shadow: "sm",
        withBorder: true,
      },
      styles: {
        root: {
          backgroundColor: rgba("#fffdf9", 0.78),
          borderColor: rgba("#24303b", 0.08),
          backdropFilter: "blur(18px)",
        },
      },
    },
    Button: {
      defaultProps: {
        radius: "xl",
      },
    },
    NativeSelect: {
      defaultProps: {
        radius: "xl",
      },
    },
    Badge: {
      defaultProps: {
        radius: "xl",
        variant: "light",
      },
    },
  },
  other: {
    shellBackground:
      "radial-gradient(circle at top left, rgba(246, 193, 119, 0.5), transparent 35%), linear-gradient(160deg, #f7f2ea 0%, #edf3f6 55%, #d7e5ec 100%)",
    heroGlow:
      "radial-gradient(circle at top, rgba(255, 253, 249, 0.85), rgba(255, 253, 249, 0.2) 68%, transparent 100%)",
  },
});
