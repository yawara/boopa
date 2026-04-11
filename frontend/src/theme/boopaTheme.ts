import { alpha, createTheme } from "@mui/material/styles";

export const boopaTheme = createTheme({
  shape: {
    borderRadius: 28,
  },
  palette: {
    primary: {
      light: "#ef9270",
      main: "#e45d2b",
      dark: "#c84310",
      contrastText: "#fffdf9",
    },
    secondary: {
      light: "#8295a8",
      main: "#4a5c6e",
      dark: "#37424f",
    },
    background: {
      default: "#edf3f6",
      paper: "#fffdf9",
    },
    text: {
      primary: "#24303b",
      secondary: "#4a5c6e",
    },
    warning: {
      main: "#dd922b",
    },
    success: {
      main: "#2f8f6b",
    },
    info: {
      main: "#3b82b8",
    },
  },
  typography: {
    fontFamily: '"M PLUS Rounded 1c", "Segoe UI", sans-serif',
    h1: {
      fontFamily: '"Baloo 2", "M PLUS Rounded 1c", sans-serif',
      fontWeight: 700,
      letterSpacing: "-0.03em",
    },
    h2: {
      fontFamily: '"Baloo 2", "M PLUS Rounded 1c", sans-serif',
      fontWeight: 700,
      letterSpacing: "-0.02em",
    },
    h3: {
      fontFamily: '"Baloo 2", "M PLUS Rounded 1c", sans-serif',
      fontWeight: 700,
    },
    h4: {
      fontFamily: '"Baloo 2", "M PLUS Rounded 1c", sans-serif',
      fontWeight: 700,
    },
    h5: {
      fontFamily: '"Baloo 2", "M PLUS Rounded 1c", sans-serif',
      fontWeight: 700,
    },
    h6: {
      fontFamily: '"Baloo 2", "M PLUS Rounded 1c", sans-serif',
      fontWeight: 700,
    },
    button: {
      fontWeight: 700,
      textTransform: "none",
    },
  },
  components: {
    MuiCssBaseline: {
      styleOverrides: {
        ":root": {
          colorScheme: "light",
        },
      },
    },
    MuiPaper: {
      defaultProps: {
        elevation: 0,
        variant: "outlined",
      },
      styleOverrides: {
        root: {
          backgroundColor: alpha("#fffdf9", 0.78),
          borderColor: alpha("#24303b", 0.08),
          backdropFilter: "blur(18px)",
          borderRadius: 36,
          boxShadow: `0 18px 40px ${alpha("#274152", 0.08)}`,
        },
      },
    },
    MuiButton: {
      styleOverrides: {
        root: {
          borderRadius: 999,
          paddingInline: 20,
          paddingBlock: 10,
          boxShadow: `0 14px 28px ${alpha("#e45d2b", 0.18)}`,
          transition: "transform 160ms ease, box-shadow 160ms ease",
          "&:hover": {
            transform: "translateY(-1px)",
            boxShadow: `0 18px 32px ${alpha("#e45d2b", 0.22)}`,
          },
        },
        contained: {
          backgroundImage: "linear-gradient(135deg, #ee8c6d 0%, #e45d2b 55%, #c84310 100%)",
        },
      },
    },
    MuiChip: {
      styleOverrides: {
        root: {
          borderRadius: 999,
          fontWeight: 700,
          textTransform: "uppercase",
          backgroundColor: alpha("#fffdf9", 0.88),
        },
      },
    },
    MuiOutlinedInput: {
      styleOverrides: {
        root: {
          borderRadius: 20,
          backgroundColor: alpha("#fffdf9", 0.94),
        },
      },
    },
    MuiInputLabel: {
      styleOverrides: {
        root: {
          fontWeight: 700,
        },
      },
    },
  },
});
