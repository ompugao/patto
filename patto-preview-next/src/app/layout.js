import Script from 'next/script';
import "./globals.css";
// Import Pure CSS modules
import 'purecss/build/base.css';
import 'purecss/build/grids.css';
import 'purecss/build/buttons.css';
import 'purecss/build/tables.css';

export const metadata = {
  description: "Preview and navigate Patto notes",
  icons: {
    icon: "/icon.png",
  },
};

export default function RootLayout({ children }) {
  return (
    <html lang="en">
      <body>
        {children}
      </body>
    </html>
  );
}
