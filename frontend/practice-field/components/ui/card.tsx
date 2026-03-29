export default function Card({
    children,
}: {
    children: React.ReactNode
}) {
    return (
        <div className="bg-panel p-6 rounded-xl border border-white/10">
            {children}
        </div>
    );
}