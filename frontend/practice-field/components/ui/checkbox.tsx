export default function Checkbox({ label, checked, onChange, checkedColor }: { label: string, checked: boolean, onChange: (checked: boolean) => void, checkedColor?: string }) {
    return (
        <div style={{ display: "flex", alignItems: "center", cursor: "pointer" }} onClick={() => onChange(!checked)}>
            <div style={{
                width: "30px",
                height: "30px",
                backgroundColor: checked ? (checkedColor || "green") : "gray",
                borderRadius: "25%",
                marginRight: "5px",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
            }}>
                {checked && (
                    <svg viewBox="0 0 12 12" width="18" height="18" fill="none" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polyline points="2,6 5,9 10,3" />
                    </svg>
                )}
            </div>
            <span>{label}</span>
        </div>
    );
}
