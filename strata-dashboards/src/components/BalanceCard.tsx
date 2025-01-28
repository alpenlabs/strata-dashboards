interface BalanceCardProps {
    title: string;
    balance: string;
}

const BalanceCard = ({ title, balance }: BalanceCardProps) => {
    return (
        <div className="balance-card">
            <span className="title">{title}</span>
            <span className="balance"><span className="unit">BTC</span> {balance}</span>
        </div>
    );
};

export default BalanceCard;