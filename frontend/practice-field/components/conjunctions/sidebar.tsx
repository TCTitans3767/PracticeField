export default function Sidebar() {
    return (
        <div className="flex flex-col gap-4 p-4">
            <a className="text-2xl font-bold">FMS Dashboard</a>
            <a/><a/>
            <a className="text-m" href="/">Home</a>
            <a className="text-m" href="/display">Display</a>
            <a className="text-m" href="/match-control">Match Control</a>
            <a className="text-m" href="/team-mgmt">Team Management</a>
            <a className="text-m" href="/vh109-kiosk">VH109 Kiosk</a>
            <a className="text-m" href="/view-logs">View Logs</a>
            <a className="text-m" href="/network-settings">Network Settings</a>
        </div>
    );
}