import Script from 'next/script';
import "./globals.css";

export const metadata = {
  title: "Patto Previewer",
  description: "Preview and navigate Patto notes",
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
