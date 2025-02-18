interface StatusCardProps {
    title: string;
    status: string;
}

const StatusCard = ({ title, status }: StatusCardProps) => {
    return (
        <div className="status-card">
            <div className="status-title">{title}</div>
            <div className={`status-badge`}>
                <div className="status-wrapper">
                    <span className="status-text">
                        {status.charAt(0).toUpperCase() + status.slice(1)}
                    </span>
                    <span className={`status-indicator ${status.toLowerCase()}`} />
                </div>
            </div>
        </div>
    );
};

export default StatusCard;
