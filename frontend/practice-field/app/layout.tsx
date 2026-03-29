import "../styles/globals.css";

import Sidebar from "@/components/conjunctions/sidebar";

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="flex h-screen text-text">
        <aside className="w-64 bg-panel p-4">
          <Sidebar />
        </aside>

        <main className="flex-1 p-6">
          {children}
        </main>
      </body>
    </html>
  );
}