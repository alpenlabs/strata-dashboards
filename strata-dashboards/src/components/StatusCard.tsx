interface StatusCardProps {
    title: string;
    status: string;
}

const StatusCard = ({ title, status }: StatusCardProps) => {
    return (
        <div className="flex justify-between p-4 border-b border-gray-700 bg-gray-800 rounded-lg">
            <span>{title}</span>
            <span className={status === "online" ? "text-green-400" : "text-red-400"}>
                {status}
            </span>
        </div>
    );
};

export default StatusCard;