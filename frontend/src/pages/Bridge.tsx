import { Suspense } from "react";
import { useLocation } from "react-router-dom";
import { useBridgeStatus, OperatorStatus, DepositInfo, WithdrawalInfo, ReimbursementInfo } from "../hooks/useBridgeStatus";

const truncateTxid = (txid: string, startLength = 6, endLength = 6) => {
    if (!txid) return "-"; // If no TXID, show "-"
    if (txid.length <= startLength + endLength) return txid; // If short, return as is
    return `${txid.slice(0, startLength)}...${txid.slice(-endLength)}`;
};

export default function Bridge() {
    const { pathname } = useLocation(); // Get current URL path
    const { data, isLoading, error } = useBridgeStatus();
    console.log(data);

    return (
        <div className="bridge-content">
            {/* Bridge Status Page */}
            {pathname === "/bridge" && (
                <div>
                    {!data || error ? <p className="error-text">Error loading data</p> : null}
                    <Suspense fallback={<p className="loading-text">Loading...</p>}>
                        {isLoading ? (
                            <p className="loading-text">Loading...</p>
                        ) : (
                            <div className="bridge-container">
                                <div className="bridge-section">
                                    <span className="bridge-title">Bridge operator status</span>
                                    { data && data.operators ? (
                                        <div className="table-wrapper">
                                            <table className="operators-table">
                                                <tbody>
                                                    {data.operators.map((operator: OperatorStatus, index: number) => (
                                                        <tr key={index} className="operators-row">
                                                            <td className="table-cell">{operator.operator_id}</td>
                                                            <td className="table-cell">{operator.operator_address}</td>
                                                            <td className="table-cell">{operator.status}</td>
                                                        </tr>
                                                    ))}
                                                </tbody>
                                            </table>
                                        </div>
                                    ) : (
                                        <p className="no-items">No bridge operators found.</p>
                                    )}
                                </div>
                                <div className="bridge-section">
                                    <span className="bridge-title">Bridge deposit status</span>
                                    { data && data.deposits ? (
                                        <div className="table-wrapper">
                                            <table className="transactions-table">
                                                <thead>
                                                    <tr className="transactions-header">
                                                        <th>Deposit Request TXID</th>
                                                        <th>Deposit TXID</th>
                                                        <th>Mint TXID</th>
                                                        <th>Status</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {data.deposits.map((deposit: DepositInfo, index: number) => (
                                                        <tr key={index} className="transactions-row">
                                                            <td className="table-cell">{truncateTxid(deposit.deposit_request_txid)}</td>
                                                            <td className="table-cell">{truncateTxid(deposit.deposit_txid)}</td>
                                                            <td className="table-cell">{truncateTxid(deposit.mint_txid)}</td>
                                                            <td className="table-cell">{deposit.status}</td>
                                                        </tr>
                                                    ))}
                                                </tbody>
                                            </table>
                                        </div>
                                    ) : (
                                        <p className="no-items">No bridge deposits found.</p>
                                    )}
                                </div>
                                <div className="bridge-section">
                                    <span className="bridge-title">Bridge withdrawal status</span>
                                    { data && data.withdrawals ? (
                                        <div className="table-wrapper">
                                            <table className="transactions-table">
                                                <thead>
                                                    <tr className="transactions-header">
                                                        <th>Withdrawal Request TXID</th>
                                                        <th>Fulfillment TXID</th>
                                                        <th>Status</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {data.withdrawals.map((withdrawal: WithdrawalInfo, index: number) => (
                                                        <tr key={index} className="transactions-row">
                                                            <td className="table-cell">{truncateTxid(withdrawal.withdrawal_request_txid)}</td>
                                                            <td className="table-cell">{truncateTxid(withdrawal.fulfillment_txid)}</td>
                                                            <td className="table-cell">{withdrawal.status}</td>
                                                        </tr>
                                                    ))}
                                                </tbody>
                                            </table>
                                        </div>
                                    ) : (
                                        <p className="no-items">No bridge withdrawals found.</p>
                                    )}
                                </div>
                                <div className="bridge-section">
                                    <span className="bridge-title">Bridge reimbursement status</span>
                                    { data && data.reimbursements ? (
                                        <div className="table-wrapper">
                                            <table className="transactions-table">
                                                <thead>
                                                    <tr className="transactions-header">
                                                        <th>Claim TXID</th>
                                                        <th>Challenge Step</th>
                                                        <th>Payout TXID</th>
                                                        <th>Status</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    {data.reimbursements.map((reimbursement: ReimbursementInfo, index: number) => (
                                                        <tr key={index} className="transactions-row">
                                                            <td className="table-cell">{truncateTxid(reimbursement.claim_txid)}</td>
                                                            <td className="table-cell">{truncateTxid(reimbursement.challenge_step)}</td>
                                                            <td className="table-cell">{truncateTxid(reimbursement.payout_txid)}</td>
                                                            <td className="table-cell">{reimbursement.status}</td>
                                                        </tr>
                                                    ))}
                                                </tbody>
                                            </table>
                                        </div>
                                    ) : (
                                        <p className="no-items">No bridge withdrawals found.</p>
                                    )}
                                </div>
                            </div>
                        )}
                    </Suspense>
                </div>
            )}
        </div>
    );
}