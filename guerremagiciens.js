/**
 *------
 * BGA framework: © Gregory Isabelli <gisabelli@boardgamearena.com> & Emmanuel Colin <ecolin@boardgamearena.com>
 * GuerreMagiciens implementation : © <Your name here> <Your email address here>
 *
 * This code has been produced on the BGA studio platform for use on http://boardgamearena.com.
 * See http://en.boardgamearena.com/#!doc/Studio for more information.
 * -----
 *
 * guerremagiciens.js
 *
 * GuerreMagiciens user interface script
 * 
 * In this file, you are describing the logic of your user interface, in Javascript language.
 *
 */

define([
    "dojo","dojo/_base/declare",
    "ebg/core/gamegui",
    "ebg/counter"
],
function (dojo, declare) {
    return declare("bgagame.guerremagiciens", ebg.core.gamegui, {
        constructor: function(){
            console.log('guerremagiciens constructor');
              
            // Here, you can init the global variables of your user interface
            // Example:
            // this.myGlobalValue = 0;

        },
        
        /*
            setup:
            
            This method must set up the game user interface according to current game situation specified
            in parameters.
            
            The method is called each time the game interface is displayed to a player, ie:
            _ when the game starts
            _ when a player refreshes the game page (F5)
            
            "gamedatas" argument contains all datas retrieved by your "getAllDatas" PHP method.
        */
        
        setup: function( gamedatas )
        {
            console.log( "Starting game setup" );
            
            // Setting up player boards
            for( var player_id in gamedatas.players )
            {
                var player = gamedatas.players[player_id];
                         
                // TODO: Setting up players boards if needed
                // Setting up players boards if needed
                var player_board_div = $('player_board_'+player_id);
                dojo.place( this.format_block('jstpl_player_board', player ), player_board_div );
            }
            
            // TODO: Set up your game interface here, according to "gamedatas"
 
            // Setup game notifications to handle (see "setupNotifications" method below)
            this.setupNotifications();

            console.log( "Ending game setup" );
        },
       

        ///////////////////////////////////////////////////
        //// Game & client states
        
        // onEnteringState: this method is called each time we are entering into a new game state.
        //                  You can use this method to perform some user interface changes at this moment.
        //
        onEnteringState: function( stateName, args )
        {
            console.log( 'Entering state: '+stateName );
            
            switch( stateName )
            {
            
            case 'TownCriersExpense':

            // console.log( args );
            
                args.args.global_expense_input_tokens.forEach(element => {
                    dojo.place( this.format_block('jstpl_global_expense_input', { amount: element.amount, id: element.id } ), 'right-area' );
                    dojo.connect( $('expense_token_t' + element.id), 'onclick', this, 'onClickExpenseCoupon' );
                });

                dojo.place( this.format_block('jstpl_global_expense_sum', {} ), 'right-area' );
                
                break;


            case 'ItemsProduction':

                $id = 1;

                args.args.forEach(element => {

                    var step;
                    for (step = 1; step <= parseInt(element.talismans_amount); step++) {

                        dojo.place( this.format_block('jstpl_produce_magical_items', 
                            { id: $id, magical_item_name: element.talisman_name } ), 'right-area' );
                            
                        dojo.connect( $('magical_item_t' + $id), 'onclick', this, 'onClickProduceMagicalItem' );  
                          
                        $id += 1;
                    }                    
                });

                break;                
             
            /* Example:
            
            case 'myGameState':
            
                // Show some HTML block at this game state
                dojo.style( 'my_html_block_id', 'display', 'block' );
                
                break;
           */
           
            case 'dummmy':
                break;
            }
        },

        // onLeavingState: this method is called each time we are leaving a game state.
        //                 You can use this method to perform some user interface changes at this moment.
        //
        onLeavingState: function( stateName )
        {
            console.log( 'Leaving state: '+stateName );
            
            switch( stateName )
            {
                 
            case 'TownCriersExpense':

                dojo.empty("right-area");
                
                break;

            /* Example:
            
            case 'myGameState':
            
                // Hide the HTML block we are displaying only during this game state
                dojo.style( 'my_html_block_id', 'display', 'none' );
                
                break;
           */
           
           
            case 'dummmy':
                break;
            }               
        }, 

        // onUpdateActionButtons: in this method you can manage "action buttons" that are displayed in the
        //                        action status bar (ie: the HTML links in the status bar).
        //        
        onUpdateActionButtons: function( stateName, args )
        {
            console.log( 'onUpdateActionButtons: '+stateName );
                      
            if( this.isCurrentPlayerActive() )
            {            
                switch( stateName )
                {

                case 'TownCriersExpense':
                    this.addActionButton( 'button_1_id', _('Finish'), 'onFinishTownCriersExpense' );
                    break;
/*               
                 Example:
 
                 case 'myGameState':
                    
                    // Add 3 action buttons in the action status bar:
                    
                    this.addActionButton( 'button_1_id', _('Button 1 label'), 'onMyMethodToCall1' ); 
                    this.addActionButton( 'button_2_id', _('Button 2 label'), 'onMyMethodToCall2' ); 
                    this.addActionButton( 'button_3_id', _('Button 3 label'), 'onMyMethodToCall3' ); 
                    break;
*/
                }
            }
        },        

        ///////////////////////////////////////////////////
        //// Utility methods
        
        /*
        
            Here, you can defines some utility methods that you can use everywhere in your javascript
            script.
        
        */


        ///////////////////////////////////////////////////
        //// Player's action
        
        /*
        
            Here, you are defining methods to handle player's action (ex: results of mouse click on 
            game objects).
            
            Most of the time, these methods:
            _ check the action is possible at this game state.
            _ make a call to the game server
        
        */

        onClickProduceMagicalItem: function( evt ){
            if( dojo.hasClass( evt.target.id, 'magical-item-selected' ) ){
                dojo.removeClass( evt.target.id, 'magical-item-selected' );
            }
            else{
                dojo.addClass( evt.target.id, 'magical-item-selected' );
            }
        },

        onClickExpenseCoupon: function( evt ){
            if( dojo.hasClass( evt.target.id, 'global-expense-coupon-selected' ) ){
                dojo.removeClass( evt.target.id, 'global-expense-coupon-selected' );
            }
            else{
                dojo.addClass( evt.target.id, 'global-expense-coupon-selected' );
            }

            var sum = 0;
            dojo.query(".global-expense-coupon-selected").forEach(function(node, index, arr){
                sum += parseInt(node.innerHTML);  // Or this.innerHTML, this.innerText
            });

            $( 'global_expense_total' ).innerHTML = sum;
        },

        onFinishTownCriersExpense: function( evt ){
            console.log( evt );

            // Preventing default browser reaction
            dojo.stopEvent( evt );

            // Check that this action is possible (see "possibleactions" in states.inc.php)
            if( ! this.checkAction( 'ItemsProduction' ) )
            {   console.log( 'ItemsProduction not available' );
                return; 
            }

            this.ajaxcall( "/guerremagiciens/guerremagiciens/actionFinishTownCriersExpense.html", 
            { 
                lock: true, 
                intGlobalExpenseTotal: $( 'global_expense_total' ).innerHTML }, 
                    this, function( result ) {
                    
                    // What to do after the server call if it succeeded
                    // (most of the time: nothing)
                    
                    }, function( is_error) {

                    // What to do after the server call in anyway (success or failure)
                    // (most of the time: nothing)

                    } );              
        },
        
        /* Example:
        
        onMyMethodToCall1: function( evt )
        {
            console.log( 'onMyMethodToCall1' );
            
            // Preventing default browser reaction
            dojo.stopEvent( evt );

            // Check that this action is possible (see "possibleactions" in states.inc.php)
            if( ! this.checkAction( 'myAction' ) )
            {   return; }

            this.ajaxcall( "/guerremagiciens/guerremagiciens/myAction.html", { 
                                                                    lock: true, 
                                                                    myArgument1: arg1, 
                                                                    myArgument2: arg2,
                                                                    ...
                                                                 }, 
                         this, function( result ) {
                            
                            // What to do after the server call if it succeeded
                            // (most of the time: nothing)
                            
                         }, function( is_error) {

                            // What to do after the server call in anyway (success or failure)
                            // (most of the time: nothing)

                         } );        
        },        
        
        */

        
        ///////////////////////////////////////////////////
        //// Reaction to cometD notifications

        /*
            setupNotifications:
            
            In this method, you associate each of your game notifications with your local method to handle it.
            
            Note: game notification names correspond to "notifyAllPlayers" and "notifyPlayer" calls in
                  your guerremagiciens.game.php file.
        
        */
        setupNotifications: function()
        {
            console.log( 'notifications subscriptions setup' );
            
            // TODO: here, associate your game notifications with local methods
            
            // Example 1: standard notification handling
            // dojo.subscribe( 'cardPlayed', this, "notif_cardPlayed" );
            
            // Example 2: standard notification handling + tell the user interface to wait
            //            during 3 seconds after calling the method in order to let the players
            //            see what is happening in the game.
            // dojo.subscribe( 'cardPlayed', this, "notif_cardPlayed" );
            // this.notifqueue.setSynchronous( 'cardPlayed', 3000 );
            // 
        },  
        
        // TODO: from this point and below, you can write your game notifications handling methods
        
        /*
        Example:
        
        notif_cardPlayed: function( notif )
        {
            console.log( 'notif_cardPlayed' );
            console.log( notif );
            
            // Note: notif.args contains the arguments specified during you "notifyAllPlayers" / "notifyPlayer" PHP call
            
            // TODO: play the card in the user interface.
        },    
        
        */
   });             
});
