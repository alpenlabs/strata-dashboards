interface BalanceCardProps {
    title: string;
    balance: string;
}

const BalanceCard = ({ title, balance }: BalanceCardProps) => {
    return (
        <div className="flex justify-between p-4 border-b border-gray-700 bg-gray-800 rounded-lg">
            <span>{title}</span>
            <span className="text-blue-400">{balance}</span>
        </div>
    );
};

export default BalanceCard;