interface StatusCardProps {
    title: string;
    status: string;
}

const StatusCard = ({ title, status }: StatusCardProps) => {
    return (
        <div className="status-section">
            <div className="status-title">{title.toUpperCase()}</div>
            <div className="status-value">
                <span className="status-text">{status.toUpperCase()}</span>
                <span className={`status-indicator ${status.toLowerCase()}`} />
            </div>
        </div>
    );
};

export default StatusCard;
