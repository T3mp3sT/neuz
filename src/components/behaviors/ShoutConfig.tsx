import ConfigLabel from '../config/ConfigLabel'
import ConfigPanel from '../config/ConfigPanel'

import NumericInput from '../config/NumericInput'
import StringList from '../config/StringList'

import { ShoutConfigModel } from '../../models/BotConfig'
import ConfigTable from '../config/ConfigTable'
import ConfigTableRow from '../config/ConfigTableRow'
import { DefaultValuesChecker } from '../utils/DefaultValuesChecker'


type Props = {
    config: ShoutConfigModel,
    onChange: (config: ShoutConfigModel) => void,
}

const ShoutConfig = ({ config, onChange }: Props) => {
    const defaultValues = {
        'shout_interval': 30000,
    }

    DefaultValuesChecker(config, defaultValues, onChange)

    return (
        <>
            <ConfigPanel>
                <ConfigTable>
                    <ConfigTableRow
                        layout="v"
                        label={<ConfigLabel name="Messages" helpText="Add messages to the list to shout multiple messages. One message per line." />}
                        item={<StringList messages={config.shout_messages ?? []} onChange={value => onChange?.({ ...config, shout_messages: value })} />}
                    />
                    <ConfigTableRow
                        layout="v"
                        label={<ConfigLabel name="Interval" helpText="Interval between shouts in milliseconds." />}
                        item={<NumericInput unit="ms" value={config.shout_interval} onChange={value => onChange?.({ ...config, shout_interval: value })} />}
                    />
                </ConfigTable>
            </ConfigPanel>
        </>
    )
}

export default ShoutConfig;


