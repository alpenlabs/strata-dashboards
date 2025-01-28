interface BalanceCardProps {
    title: string;
    balance: string;
}

const BalanceCard = ({ title, balance }: BalanceCardProps) => {
    return (
        <div className="balance-card">
            <span className="font-semibold">{title}</span>
            <span className="font-medium">{balance}</span>
        </div>
    );
};

export default BalanceCard;