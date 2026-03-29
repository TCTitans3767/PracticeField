"use client"

import "../../styles/globals.css";

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="flex h-screen text-text">
        <main className="flex-1 p-6">
          {children}
        </main>
        <button className="fixed bottom-4 right-4 z-50 bg-black text-white px-4 py-2 rounded-lg font-bold" onClick={() => window.location.href = "/"}>
            Home
        </button>
      </body>
    </html>
  );
}