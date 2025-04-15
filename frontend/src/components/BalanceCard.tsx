import "../styles/network.css";

interface BalanceCardProps {
    title: string;
    balance: string;
}

const BalanceCard = ({ title, balance }: BalanceCardProps) => {
    return (
        <div className="balance-item">
            <span className="balance-title">{title}</span>
            <span className="balance-value"><span>BTC</span> {balance}</span>
        </div>
    );
};

export default BalanceCard;