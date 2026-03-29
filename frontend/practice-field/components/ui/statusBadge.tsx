export default function StatusBadge({title, status}: {title: string, status: string}) {
    // colored box based on status with title next to it
    let color = "gray";
    if (status == "ok") {
        color = "green";
    } else if (status == "warn") {
        color = "yellow";
    } else if (status == "err") {
        color = "red";
    }
    return (
        <div style={{display: "flex", alignItems: "center"}}>
            <div style={{width: "30px", height: "30px", backgroundColor: color, borderRadius: "25%", marginRight: "5px"}}></div>
            <span>{title}</span>
        </div>
    );
}